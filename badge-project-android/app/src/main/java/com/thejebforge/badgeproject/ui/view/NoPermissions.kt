package com.thejebforge.badgeproject.ui.view

import android.content.res.Configuration
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.thejebforge.badgeproject.R
import com.thejebforge.badgeproject.ui.theme.BadgeProjectTheme

object NoPermissions : IRoute {
    override val name: String
        get() = "no_perms"
}

@Composable
fun NoPermissionsScreen(requestPermissions: () -> Unit) {
    Surface {
        Column(
            Modifier.fillMaxSize(),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                stringResource(R.string.no_permissions_title),
                style = MaterialTheme.typography.displaySmall
            )
            Text(
                stringResource(R.string.no_permissions_text),
                Modifier.padding(20.dp),
                style = MaterialTheme.typography.bodyMedium,
                textAlign = TextAlign.Left
            )
            Button(onClick = {
                requestPermissions()
            }) {
                Text(stringResource(R.string.authorize))
            }
        }
    }
}

@Preview(
    showSystemUi = true,
    uiMode = Configuration.UI_MODE_NIGHT_YES
)
@Composable
private fun Preview() {
    BadgeProjectTheme {
        NoPermissionsScreen{}
    }
}