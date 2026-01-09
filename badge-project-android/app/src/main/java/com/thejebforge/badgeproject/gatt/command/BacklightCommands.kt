package com.thejebforge.badgeproject.gatt.command

import com.thejebforge.badgeproject.gatt.GATTHelper
import com.thejebforge.badgeproject.util.BoardConstants

fun GATTHelper.getBacklightState(callback: (Boolean?) -> Unit) = this.apply {
    readCharacteristicToBool(BoardConstants.SCREEN_SVC, BoardConstants.BACKLIGHT_CHR, callback)
}

fun GATTHelper.setBacklightState(newState: Boolean, onDone: (Boolean) -> Unit) = this.apply {
    writeCharacteristicBoolean(BoardConstants.SCREEN_SVC, BoardConstants.BACKLIGHT_CHR, newState, onDone)
}