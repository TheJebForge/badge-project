package com.thejebforge.badgeproject.gatt.command

import android.Manifest
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothStatusCodes
import android.os.Handler
import androidx.annotation.RequiresPermission
import com.thejebforge.badgeproject.gatt.GATTCommandHandler
import com.thejebforge.badgeproject.gatt.command.ActionResponse.Success
import com.thejebforge.badgeproject.gatt.getCharacteristic
import com.thejebforge.badgeproject.util.BoardConstants

const val ACTION_TIMEOUT = 1000L

abstract class ActionCommand(
    gattServer: BluetoothGatt,
    commandHandler: GATTCommandHandler,
    protected val handler: Handler
) : GATTCommand(gattServer, commandHandler) {
    abstract fun payload(): ByteArray
    abstract fun received(data: ByteArray)
    abstract fun failed(reason: ActionResponse<Nothing>)

    private lateinit var cmdChr: BluetoothGattCharacteristic
    private lateinit var respChr: BluetoothGattCharacteristic
    private var operation: Byte = 0

    private var dataReceived = false

    @RequiresPermission(Manifest.permission.BLUETOOTH_CONNECT)
    override fun runCommand() {
        val cmdChr = gattServer.getCharacteristic(BoardConstants.CHARACTER_SVC, BoardConstants.COMMAND_CHR)
        val respChr = gattServer.getCharacteristic(BoardConstants.CHARACTER_SVC, BoardConstants.RESPONSE_CHR)

        if (cmdChr == null || respChr == null) {
            failed(ActionResponse.CharacteristicsNotFound)
            continueExecution()
            return
        }

        this.cmdChr = cmdChr
        this.respChr = respChr

        val payload = payload()

        operation = payload[1]

        if (gattServer.writeCharacteristic(
                cmdChr,
                payload,
                BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE
            ) != BluetoothStatusCodes.SUCCESS
        ) {
            failed(ActionResponse.FailedWrite)
            continueExecution()
            return
        }

        handler.postDelayed({
            if (!dataReceived) {
                failed(ActionResponse.TimedOut)
                continueExecution()
            }
        }, ACTION_TIMEOUT)
    }

    override fun onCharacteristicChanged(
        characteristic: BluetoothGattCharacteristic,
        value: ByteArray
    ) {
        if (characteristic != respChr) return
        if (operation != value[1]) return

        val success = value[0] == 1.toByte()

        dataReceived = true

        if (success) {
            received(value.sliceArray(2..<value.size))
        } else {
            failed(ActionResponse.ReportedFailure)
        }

        continueExecution()
    }
}

sealed class ActionResponse<out T> {
    data class Success<T>(
        val value: T
    ) : ActionResponse<T>() {
        override fun reason(): String = ""
    }

    data object CharacteristicsNotFound : ActionResponse<Nothing>() {
        override fun reason(): String = "Characteristics Not Found"
    }

    data object FailedWrite : ActionResponse<Nothing>() {
        override fun reason(): String = "Failed Write"
    }

    data object TimedOut : ActionResponse<Nothing>() {
        override fun reason(): String = "Timed Out"
    }

    data object ReportedFailure : ActionResponse<Nothing>() {
        override fun reason(): String = "Reported Failure"
    }

    fun <R> fold(onSuccess: (value: T) -> R, onFailure: (reason: ActionResponse<Nothing>) -> R): R =
        when (this) {
            is Success -> onSuccess(value)
            is CharacteristicsNotFound -> onFailure(this)
            is FailedWrite -> onFailure(this)
            is ReportedFailure -> onFailure(this)
            is TimedOut -> onFailure(this)
        }

    val isSuccess = this is Success<T>
    val isFailure = !isSuccess

    fun getOrNull(): T? = fold({ it }, { null })

    abstract fun reason(): String
}

fun <T> T.asSuccess(): ActionResponse<T> = Success(this)