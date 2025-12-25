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

fun GATTHelper.getActionCount(callback: (Int?) -> Unit) = this.apply {
    withService(BoardConstants.CHARACTER_SVC) {
        readCharacteristic(BoardConstants.ACTION_COUNT_CHR) {
            (chr, data) ->
            if (chr == null || data == null) {
                callback(null)
                return@readCharacteristic
            }

            val buffer = ByteBuffer.wrap(data)
            buffer.order(ByteOrder.LITTLE_ENDIAN)
            callback(buffer.getInt())
        }
    }
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