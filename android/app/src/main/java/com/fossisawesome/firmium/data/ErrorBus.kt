package com.fossisawesome.firmium.data

import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow

/** App-wide one-shot error notifications. replay=0 so a stale error is never
 *  re-shown on recomposition; a small buffer survives brief collector gaps. */
class ErrorBus {
    private val _events = MutableSharedFlow<UserError>(
        replay = 0,
        extraBufferCapacity = 8,
        onBufferOverflow = BufferOverflow.DROP_OLDEST,
    )
    val events: SharedFlow<UserError> = _events.asSharedFlow()

    /** Non-suspending; safe to call from any coroutine or catch block. */
    fun report(error: UserError) {
        _events.tryEmit(error)
    }
}
