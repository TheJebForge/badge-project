package com.thejebforge.badgeproject.gatt.command

import android.bluetooth.BluetoothGatt
import android.os.Handler
import com.thejebforge.badgeproject.gatt.GATTCommandHandler
import com.thejebforge.badgeproject.gatt.GATTHelper
import com.thejebforge.badgeproject.util.PayloadCreator

class InvokeAction(
    gattServer: BluetoothGatt,
    commandHandler: GATTCommandHandler,
    handler: Handler,
    private val id: String,
    private val onDone: (Boolean) -> Unit
) : ActionCommand(gattServer, commandHandler, handler) {
    override fun payload(): ByteArray = PayloadCreator.invokeAction(id)

    override fun received(data: ByteArray) {
        onDone(true)
    }

    override fun failed(reason: ActionResponse<Nothing>) {
        onDone(false)
    }
}

fun GATTHelper.invokeAction(id: String, callback: (Boolean) -> Unit) = this.apply {
    commandHandler.appendCommand(InvokeAction(
        gattServer,
        commandHandler,
        handler,
        id,
        callback
    ))
}