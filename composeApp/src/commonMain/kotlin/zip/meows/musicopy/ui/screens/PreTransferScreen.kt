package zip.meows.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import uniffi.musicopy.ClientModel
import zip.meows.musicopy.ui.components.SectionCard

@Composable
fun PreTransferScreen(
    clientModel: ClientModel,
    onCancel: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxSize(),
        verticalArrangement = Arrangement.Center,
    ) {
        SectionCard(
            title = "Pretransfer",
            body = {
                Text("Press Accept on the other end to continue.")
            },
            onCancel = onCancel,
        )
    }
}
