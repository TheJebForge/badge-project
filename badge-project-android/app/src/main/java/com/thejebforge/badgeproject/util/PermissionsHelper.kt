package com.thejebforge.badgeproject.util

import android.app.Activity
import android.content.Context
import android.content.pm.PackageManager
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat

class PermissionsHelper {
    companion object {
        fun gotPermissions(context: Context, permissions: Array<String>): Boolean {
            for (perm in permissions) {
                if (ContextCompat.checkSelfPermission(context, perm) != PackageManager.PERMISSION_GRANTED) {
                    return false
                }
            }

            return true
        }

        fun shouldShowRationale(activity: Activity, permissions: Array<String>): Boolean {
            for (perm in permissions) {
                if (ActivityCompat.shouldShowRequestPermissionRationale(activity, perm)) {
                    return true
                }
            }

            return false
        }
    }
}
