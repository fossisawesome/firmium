package com.fossisawesome.firmium.data

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.net.SocketTimeoutException
import java.net.UnknownHostException

class UserErrorTest {
    @Test fun unknownHostIsNetwork() {
        assertEquals(UserError.Network, UnknownHostException("no dns").toUserError())
    }
    @Test fun socketTimeoutIsTimeout() {
        assertEquals(UserError.Timeout, SocketTimeoutException("slow").toUserError())
    }
    @Test fun http404IsNotFound() {
        assertEquals(UserError.NotFound, HttpStatusException(404).toUserError())
    }
    @Test fun http500IsServer() {
        assertEquals(UserError.Server(500), HttpStatusException(500).toUserError())
    }
    @Test fun everyCategoryHasMessage() {
        listOf(
            UserError.Network, UserError.Timeout, UserError.Auth,
            UserError.NotFound, UserError.Server(500), UserError.Storage, UserError.Unknown
        ).forEach { assertTrue(it.message.isNotBlank()) }
    }
}
