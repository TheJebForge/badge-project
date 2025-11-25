package com.thejebforge.badgeproject.data.intermediate

import kotlinx.serialization.Serializable

@Serializable
data class Device(
    val name: String,
    val mac: String
)
