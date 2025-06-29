package zip.meows.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import zip.meows.musicopy.ui.QRScanner
import zip.meows.musicopy.ui.components.SectionCard

@Composable
fun ConnectingScreen(
    onCancel: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxSize(),
        verticalArrangement = Arrangement.Center,
    ) {
        SectionCard(
            title = "Connecting",
            body = {
                Text("beep boop")
            },
            onCancel = onCancel,
        )
    }
}
