package com.thejebforge.badgeproject.gatt

import com.thejebforge.badgeproject.gatt.command.ActionResponse
import com.thejebforge.badgeproject.gatt.command.asSuccess
import com.thejebforge.badgeproject.gatt.command.getActionId
import com.thejebforge.badgeproject.gatt.command.getActionName
import com.thejebforge.badgeproject.gatt.command.readCharacteristic
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

fun ByteArray.toStringWithoutNulls(): String = String(this).trim(Char(0))

fun GATTHelper.readCharacteristicToString(svc: String, chr: String, callback: (String?) -> Unit) {
    withService(svc) {
        readCharacteristic(chr) {
                (chr, data) ->
            if (chr == null || data == null) {
                callback(null)
                return@readCharacteristic
            }

            callback(data.toStringWithoutNulls())
        }
    }
}

fun GATTHelper.readCharacteristicToInt(svc: String, chr: String, callback: (Int?) -> Unit) {
    withService(svc) {
        readCharacteristic(chr) {
                (chr, data) ->
            if (chr == null || data == null) {
                callback(null)
                return@readCharacteristic
            }

            callback(data.toInt())
        }
    }
}

fun GATTHelper.getDeviceMode(callback: (Int?) -> Unit) = this.apply {
    readCharacteristicToInt(BoardConstants.CHARACTER_SVC, BoardConstants.CURRENT_MODE_CHR, callback)
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

fun GATTHelper.getActionList(callback: (List<ActionResponse<Pair<String, String>>>?) -> Unit) {
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