package com.fossisawesome.firmium.data

import java.io.IOException
import java.net.SocketTimeoutException
import java.net.UnknownHostException
import java.security.GeneralSecurityException

/** Thrown by ApiClient on a non-2xx HTTP response, carrying the status code. */
class HttpStatusException(val code: Int) : IOException("HTTP $code")

/** A user-facing error category. `message` is the single source of wording. */
sealed class UserError {
    object Network : UserError()
    object Timeout : UserError()
    object Auth : UserError()
    object NotFound : UserError()
    data class Server(val code: Int) : UserError()
    object Storage : UserError()
    object Unknown : UserError()

    val message: String
        get() = when (this) {
            Network -> "Can't reach your server. Check your connection."
            Timeout -> "The server took too long to respond. Try again."
            Auth -> "Login failed. Check your username and password."
            NotFound -> "That item couldn't be found on the server."
            is Server -> "The server reported a problem (code $code). Try again later."
            Storage -> "Couldn't access your device's secure storage."
            Unknown -> "Something went wrong. Please try again."
        }
}

fun Throwable.toUserError(): UserError = when (this) {
    is HttpStatusException -> when (code) {
        401, 403 -> UserError.Auth
        404 -> UserError.NotFound
        else -> if (code >= 500) UserError.Server(code) else UserError.Unknown
    }
    is UnknownHostException -> UserError.Network
    is SocketTimeoutException -> UserError.Timeout
    is GeneralSecurityException -> UserError.Storage
    is IOException -> UserError.Network
    else -> UserError.Unknown
}
