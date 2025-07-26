package zip.meows.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import zip.meows.musicopy.ui.QRScanner
import zip.meows.musicopy.ui.components.SectionCard
import zip.meows.musicopy.ui.components.TopBar

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
                title = "Connect using QR code",
                onShowNodeStatus = onShowNodeStatus,
                onBack = onCancel
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            verticalArrangement = Arrangement.Center,
        ) {
            SectionCard(
                onCancel = onCancel,
                title = "Connect using QR code",
                body = {
                    Text("Scan the QR code etc etc etc.")

                    Text("TODO: Desktop install link")

                    Column(
                        modifier = Modifier.fillMaxWidth().padding(vertical = 32.dp),
                        horizontalAlignment = Alignment.CenterHorizontally
                    ) {
                        QRScanner(onResult = { nodeId ->
                            onSubmit(nodeId)
                        })
                    }
                })
        }
    }
}
