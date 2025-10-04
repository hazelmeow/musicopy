package app.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import app.musicopy.mockClientModel
import app.musicopy.shortenNodeId
import app.musicopy.ui.components.TopBar
import uniffi.musicopy.ClientModel

@Composable
fun WaitingScreen(
    snackbarHost: @Composable () -> Unit,
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
        },
        snackbarHost = snackbarHost,
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding).padding(8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                "Connected to ${shortenNodeId(clientModel.nodeId)}",
                style = MaterialTheme.typography.headlineSmall
            )

            Text(
                "Press Accept on the other device to continue.",
                style = MaterialTheme.typography.bodyLarge
            )

            Button(
                onClick = onCancel,
            ) {
                Text("Cancel")
            }
        }
    }
}

@Composable
fun WaitingScreenSandbox() {
    WaitingScreen(
        snackbarHost = {},
        onShowNodeStatus = {},

        clientModel = mockClientModel(),
        onCancel = {},
    )
}
