package com.thejebforge.badgeproject

import android.app.Application
import android.app.NotificationChannel
import android.app.NotificationManager
import androidx.core.content.getSystemService
import dagger.hilt.android.HiltAndroidApp

@HiltAndroidApp
class MainApplication : Application() {
    companion object {
        const val BOARD_NOTIFICATION_CHANNEL: String = "board_connection"
    }

    override fun onCreate() {
        super.onCreate()

        val channel = NotificationChannel(
            BOARD_NOTIFICATION_CHANNEL,
            getString(R.string.notification_channel),
            NotificationManager.IMPORTANCE_LOW
        )
        val manager = getSystemService<NotificationManager>()!!
        manager.createNotificationChannel(channel)
    }
}