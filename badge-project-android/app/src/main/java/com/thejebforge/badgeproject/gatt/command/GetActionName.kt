package com.thejebforge.badgeproject.gatt.command

import android.bluetooth.BluetoothGatt
import android.os.Handler
import com.thejebforge.badgeproject.gatt.GATTCommandHandler
import com.thejebforge.badgeproject.gatt.GATTHelper
import com.thejebforge.badgeproject.util.PayloadCreator

class GetActionName(
    gattServer: BluetoothGatt,
    commandHandler: GATTCommandHandler,
    handler: Handler,
    private val id: String,
    private val onDone: (ActionResponse<String>) -> Unit
) : ActionCommand(gattServer, commandHandler, handler) {
    override fun payload(): ByteArray = PayloadCreator.getActionDisplayName(id)

    override fun received(data: ByteArray) {
        onDone(String(data).trim(Char(0)).asSuccess())
    }

    override fun failed(reason: ActionResponse<Nothing>) {
        onDone(reason)
    }
}

fun GATTHelper.getActionName(id: String, callback: (ActionResponse<String>) -> Unit) = this.apply {
    commandHandler.appendCommand(GetActionName(
        gattServer,
        commandHandler,
        handler,
        id,
        callback
    ))
}