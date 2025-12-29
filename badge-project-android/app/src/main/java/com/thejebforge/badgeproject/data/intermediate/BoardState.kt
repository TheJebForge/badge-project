package com.thejebforge.badgeproject.data.intermediate

import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.snapshots.SnapshotStateList

enum class BoardMode {
    CHARACTER;

    companion object {
        fun fromInt(value: Int): BoardMode? = when(value) {
            0 -> CHARACTER
            else -> null
        }
    }

    override fun toString(): String {
        return when(this) {
            CHARACTER -> "Character"
        }
    }
}

data class CharacterAction(
    val id: String,
    val name: String
)

data class CharacterInfo(
    val mode: MutableState<BoardMode> = mutableStateOf(BoardMode.CHARACTER),
    val name: MutableState<String?> = mutableStateOf(null),
    val species: MutableState<String?> = mutableStateOf(null),
    val actions: SnapshotStateList<CharacterAction> = mutableStateListOf()
)

sealed class CharacterState {
    data class Loaded(val info: CharacterInfo) : CharacterState()
    data object Failed : CharacterState()
    data object Loading : CharacterState()
}

data class BoardState(
    val currentDevice: MutableState<Device?> = mutableStateOf(null),
    val deviceConnected: MutableState<Boolean> = mutableStateOf(false),
    val character: MutableState<CharacterState> = mutableStateOf(CharacterState.Loading),
    val hadConnection: MutableState<Boolean> = mutableStateOf(false)
)