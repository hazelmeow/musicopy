package app.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import app.musicopy.mockNodeId
import app.musicopy.shortenNodeId
import app.musicopy.ui.components.LoadingButton
import app.musicopy.ui.components.TopBar

@Composable
fun DisconnectedScreen(
    onShowNodeStatus: () -> Unit,

    nodeId: String,
    isConnecting: Boolean,
    onCancel: () -> Unit,
    onReconnect: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopBar(
                title = "Disconnected",
                onShowNodeStatus = onShowNodeStatus,
                onBack = onCancel
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding).padding(8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                "Lost connection to ${shortenNodeId(nodeId)}",
                style = MaterialTheme.typography.headlineSmall
            )

            LoadingButton(
                label = "Reconnect",
                onClick = onReconnect,
                loading = isConnecting,
            )
        }
    }
}

@Composable
fun DisconnectedScreenSandbox() {
    DisconnectedScreen(
        onShowNodeStatus = {},

        nodeId = mockNodeId(),
        isConnecting = false,
        onCancel = {},
        onReconnect = {}
    )
}
