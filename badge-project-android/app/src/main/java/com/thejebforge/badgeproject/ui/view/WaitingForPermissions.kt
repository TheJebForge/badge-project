package com.thejebforge.badgeproject.ui.view

import android.content.res.Configuration
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.sp
import com.thejebforge.badgeproject.R
import com.thejebforge.badgeproject.ui.theme.BadgeProjectTheme

object WaitingForPermissions : IRoute {
    override val name: String
        get() = "waiting_perms"
}

@Composable
fun WaitingForPermissionsScreen(checkPerms: () -> Unit) {
    Surface {
        Column(
            Modifier
                .fillMaxSize(),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                stringResource(R.string.waiting_for_permissions),
                fontSize = 24.sp
            )
        }
    }
    checkPerms()
}

@Preview(
    showSystemUi = true,
    uiMode = Configuration.UI_MODE_NIGHT_YES
)
@Composable
private fun Preview() {
    BadgeProjectTheme {
        WaitingForPermissionsScreen {

        }
    }
}