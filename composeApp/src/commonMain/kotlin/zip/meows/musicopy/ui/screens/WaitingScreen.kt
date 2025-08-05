package zip.meows.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import uniffi.musicopy.ClientModel
import zip.meows.musicopy.shortenNodeId
import zip.meows.musicopy.ui.QRScanner
import zip.meows.musicopy.ui.components.Info
import zip.meows.musicopy.ui.components.SectionCard
import zip.meows.musicopy.ui.components.TopBar

@Composable
fun WaitingScreen(
    onShowNodeStatus: () -> Unit,

    clientModel: ClientModel,
    onCancel: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopBar(
                title = "Waiting to connect",
                onShowNodeStatus = onShowNodeStatus,
                onBack = onCancel
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding).padding(8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Text(
                "Connected to ${shortenNodeId(clientModel.nodeId)}",
                style = MaterialTheme.typography.bodyLarge
            )

            Text(
                "Press Accept on the other device to continue.",
                style = MaterialTheme.typography.bodyLarge
            )

            Info {
                Text("lorem")
            }
        }
    }
}
