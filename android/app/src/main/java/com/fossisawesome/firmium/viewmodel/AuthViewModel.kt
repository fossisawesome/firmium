package com.fossisawesome.firmium.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.fossisawesome.firmium.FirmiumApplication
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.api.AuthManager
import com.fossisawesome.firmium.data.toUserError
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

data class AuthState(
    val isAuthenticated: Boolean = false,
    val isLoading: Boolean = true,
    val error: String? = null,
    val savedServer: String = "",
    val savedUsername: String = "",
    val savePassword: Boolean = true,
    val needsLogin: Boolean = false,
    val savedServers: List<AuthManager.SavedServer> = emptyList(),
)

class AuthViewModel(app: Application) : AndroidViewModel(app) {

    private val auth: AuthManager = getApplication<FirmiumApplication>().auth
    private val api: ApiClient = getApplication<FirmiumApplication>().api
    private val prefs = getApplication<FirmiumApplication>().prefs

    private val _state = MutableStateFlow(AuthState())
    val state: StateFlow<AuthState> = _state.asStateFlow()

    init {
        viewModelScope.launch {
            val server = prefs.serverUrl.first() ?: ""
            val username = prefs.username.first() ?: ""
            val savePass = prefs.savePasswordEnabled.first()
            val autoLogin = prefs.autoLoginEnabled.first()

            val (restored, needsLogin) = if (autoLogin && server.isNotEmpty() && username.isNotEmpty()) {
                try {
                    val ok = auth.tryRestoreCredentials()
                    // Credentials were expected on disk but couldn't be loaded → prompt re-login.
                    Pair(ok, !ok)
                } catch (e: Exception) {
                    Pair(false, true)
                }
            } else {
                Pair(false, false)
            }

            val servers = auth.savedServers()
            _state.value = AuthState(
                isAuthenticated = restored,
                isLoading = false,
                savedServer = server,
                savedUsername = username,
                savePassword = savePass,
                needsLogin = needsLogin,
                savedServers = servers,
            )
        }
    }

    // Performs a test API call to verify the credentials work before persisting.
    fun login(server: String, username: String, password: String, savePassword: Boolean) {
        _state.value = _state.value.copy(isLoading = true, error = null)
        viewModelScope.launch {
            try {
                // Temporarily set credentials so the API call can use them.
                auth.setCredentials(server, username, password)
                // Verify by fetching artists — lightweight and requires valid auth.
                api.getArtists()
                // Persist now that we know they work.
                auth.persistCredentials(server, username, password, savePassword)
                prefs.setSavePasswordEnabled(savePassword)
                _state.value = AuthState(isAuthenticated = true, isLoading = false, needsLogin = false)
            } catch (e: Exception) {
                if (e is kotlinx.coroutines.CancellationException) throw e
                auth.clearCredentials()
                _state.value = AuthState(
                    isAuthenticated = false,
                    isLoading = false,
                    savedServer = server,
                    savedUsername = username,
                    savePassword = savePassword,
                    error = when {
                        e.message?.contains("Unable to resolve host") == true ->
                            "Can't reach server — check the URL"
                        e.message?.contains("40") == true || e.message?.contains("wrong") == true ->
                            "Wrong username or password"
                        e.message?.contains("Unable to parse TLS") == true ||
                        e.message?.contains("SSLHandshakeException") == true ->
                            "${e.toUserError().message}\n\nTry using http:// instead of https:// — your server may not have TLS enabled."
                        else -> e.toUserError().message
                    }
                )
            }
        }
    }

    fun switchToServer(url: String, username: String) {
        _state.value = _state.value.copy(isLoading = true, error = null)
        viewModelScope.launch {
            try {
                val ok = auth.switchToSaved(url, username)
                if (!ok) {
                    _state.value = _state.value.copy(isLoading = false, error = "Saved password not found — log in again")
                    return@launch
                }
                api.getArtists()
                val servers = auth.savedServers()
                _state.value = AuthState(isAuthenticated = true, isLoading = false, savedServers = servers)
            } catch (e: Exception) {
                if (e is kotlinx.coroutines.CancellationException) throw e
                auth.clearCredentials()
                _state.value = _state.value.copy(isLoading = false, error = "Switch failed: ${e.toUserError().message}")
            }
        }
    }

    fun removeServer(url: String, username: String) {
        viewModelScope.launch {
            auth.removeFromServerList(url, username)
            _state.value = _state.value.copy(savedServers = auth.savedServers())
        }
    }

    fun logout() {
        viewModelScope.launch {
            getApplication<FirmiumApplication>().prefs.clear()
            auth.clearCredentials()
            _state.value = AuthState(isAuthenticated = false, isLoading = false)
        }
    }
}
