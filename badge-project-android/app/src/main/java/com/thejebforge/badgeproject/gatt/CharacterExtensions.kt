package com.thejebforge.badgeproject.gatt

import com.thejebforge.badgeproject.gatt.command.ActionResponse
import com.thejebforge.badgeproject.gatt.command.asSuccess
import com.thejebforge.badgeproject.gatt.command.getActionId
import com.thejebforge.badgeproject.gatt.command.getActionName
import com.thejebforge.badgeproject.gatt.command.getCharacter
import com.thejebforge.badgeproject.gatt.command.readCharacteristic
import com.thejebforge.badgeproject.gatt.command.readCharacteristicToInt
import com.thejebforge.badgeproject.gatt.command.readCharacteristicToString
import com.thejebforge.badgeproject.gatt.command.withService
import com.thejebforge.badgeproject.util.BoardConstants
import com.thejebforge.badgeproject.util.callbackCollector
import java.nio.ByteBuffer
import java.nio.ByteOrder

fun ByteArray.toInt(): Int {
    val buffer = ByteBuffer.wrap(this)
    buffer.order(ByteOrder.LITTLE_ENDIAN)
    return buffer.getInt()
}

fun ByteArray.toBoolean(): Boolean {
    val buffer = ByteBuffer.wrap(this)
    buffer.order(ByteOrder.LITTLE_ENDIAN)
    return buffer.get() != 0.toByte()
}

fun ByteArray.toStringWithoutNulls(): String = String(this).trim(Char(0))

fun GATTHelper.getDeviceMode(callback: (Int?) -> Unit) = this.apply {
    readCharacteristicToInt(BoardConstants.CHARACTER_SVC, BoardConstants.CURRENT_MODE_CHR, callback)
}

fun GATTHelper.getCharacterId(callback: (String?) -> Unit) = this.apply {
    readCharacteristicToString(BoardConstants.CHARACTER_SVC, BoardConstants.CHARACTER_ID_CHR, callback)
}

fun GATTHelper.getCharacterName(callback: (String?) -> Unit) = this.apply {
    readCharacteristicToString(BoardConstants.CHARACTER_SVC, BoardConstants.CHARACTER_NAME_CHR, callback)
}

fun GATTHelper.getCharacterSpecies(callback: (String?) -> Unit) = this.apply {
    readCharacteristicToString(BoardConstants.CHARACTER_SVC, BoardConstants.CHARACTER_SPECIES_CHR, callback)
}

fun GATTHelper.getActionCount(callback: (Int?) -> Unit) = this.apply {
    readCharacteristicToInt(BoardConstants.CHARACTER_SVC, BoardConstants.ACTION_COUNT_CHR, callback)
}

fun GATTHelper.getActionList(callback: (List<ActionResponse<Pair<String, String>>>?) -> Unit) = this.apply {
    getActionCount {
        count ->
        if (count == null) {
            callback(null)
            return@getActionCount
        }

        callbackCollector(
            0..<count,
            {
                element, collect ->
                getActionId(element) {
                    idResult ->
                    idResult.fold({
                        id ->

                        getActionName(id) {
                            nameResult ->

                            nameResult.fold({
                                name ->
                                collect(Pair(id, name).asSuccess())
                            }, collect)
                        }
                    }, collect)
                }
            },
            callback
        )
    }
}

fun GATTHelper.getCharacterCount(callback: (Int?) -> Unit) = this.apply {
    readCharacteristicToInt(BoardConstants.CHARACTER_SVC, BoardConstants.CHARACTER_COUNT_CHR, callback)
}

fun GATTHelper.getCharacterList(callback: (List<ActionResponse<String>>?) -> Unit) = this.apply {
    getCharacterCount {
        count ->
        if (count == null) {
            callback(null)
            return@getCharacterCount
        }

        callbackCollector(
            0..<count,
            {
                element, collect ->
                getCharacter(element) {
                    idResult ->
                    idResult.fold({
                        id ->
                        collect(id.asSuccess())
                    }, collect)
                }
            },
            callback
        )
    }
}