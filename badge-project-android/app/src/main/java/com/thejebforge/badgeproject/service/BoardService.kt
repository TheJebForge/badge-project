package com.thejebforge.badgeproject.service

import android.app.Service
import android.os.Binder
import com.thejebforge.badgeproject.data.intermediate.BoardState
import com.thejebforge.badgeproject.data.intermediate.Device
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonClassDiscriminator


abstract class BoardService : Service() {

    val state = BoardState()

    abstract fun invokeAction(id: String)

    abstract fun toggleBacklight(onDone: () -> Unit)

    abstract fun switchCharacter(id: String)

    @OptIn(ExperimentalSerializationApi::class)
    @JsonClassDiscriminator("type")
    @Serializable
    sealed class StartAction {
        @Serializable
        @SerialName("connect")
        data class Connect(val device: Device) : StartAction()

        @Serializable
        @SerialName("stop")
        data object Stop : StartAction()

        fun serialize(): String = Json.encodeToString<StartAction>(this)
    }

    val binder = BoardServiceBinder()

    inner class BoardServiceBinder : Binder() {
        var connectedAction: ((Boolean) -> Unit)? = null
        var stopAction: (() -> Unit)? = null
        fun getService(): BoardService = this@BoardService
    }
}