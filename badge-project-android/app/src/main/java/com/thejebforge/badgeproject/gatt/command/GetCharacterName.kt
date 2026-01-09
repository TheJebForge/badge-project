package com.thejebforge.badgeproject.gatt.command

import android.bluetooth.BluetoothGatt
import android.os.Handler
import com.thejebforge.badgeproject.gatt.GATTCommandHandler
import com.thejebforge.badgeproject.gatt.GATTHelper
import com.thejebforge.badgeproject.gatt.toStringWithoutNulls
import com.thejebforge.badgeproject.util.PayloadCreator

class GetCharacterName(
    gattServer: BluetoothGatt,
    commandHandler: GATTCommandHandler,
    handler: Handler,
    private val index: UShort,
    private val onDone: (ActionResponse<String>) -> Unit
) : ActionCommand(gattServer, commandHandler, handler) {
    override fun payload(): ByteArray = PayloadCreator.getCharacterName(index)

    override fun received(data: ByteArray) {
        onDone(data.toStringWithoutNulls().asSuccess())
    }

    override fun failed(reason: ActionResponse<Nothing>) {
        onDone(reason)
    }
}

fun GATTHelper.getCharacterName(index: Int, callback: (ActionResponse<String>) -> Unit) = this.apply {
    commandHandler.appendCommand(GetCharacterName(
        gattServer,
        commandHandler,
        handler,
        index.toUShort(),
        callback
    ))
}