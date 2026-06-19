package com.fossisawesome.firmium.data.eq

/** A single equalizer band. `q` is only meaningful for parametric profiles. */
data class EqBand(val freq: Float, val gain: Float, val q: Float? = null)

/** A named equalizer profile. `mode` is "graphic" or "parametric". */
data class EqProfile(val name: String, val mode: String, val bands: List<EqBand>)
