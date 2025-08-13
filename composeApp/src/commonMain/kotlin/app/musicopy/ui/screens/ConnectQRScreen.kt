package app.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import app.musicopy.ui.QRScanner
import app.musicopy.ui.components.Info
import app.musicopy.ui.components.TopBar

@Composable
fun ConnectQRScreen(
    onShowNodeStatus: () -> Unit,

    isConnecting: Boolean,
    onSubmit: (String) -> Unit,
    onCancel: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopBar(
                title = "Scan QR code",
                onShowNodeStatus = onShowNodeStatus,
                onBack = onCancel
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding).padding(8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.Center) {
                QRScanner(onResult = { nodeId ->
                    onSubmit(nodeId)
                })
            }

            Info {
                Text("lorem")
            }

            Info {
                Text("desktop install link >")
            }
        }
    }
}
