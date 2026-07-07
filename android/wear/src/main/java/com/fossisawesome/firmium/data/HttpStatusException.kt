package com.fossisawesome.firmium.data

import java.io.IOException

/** Thrown by ApiClient on a non-2xx HTTP response, carrying the status code. */
class HttpStatusException(val code: Int) : IOException("HTTP $code")
