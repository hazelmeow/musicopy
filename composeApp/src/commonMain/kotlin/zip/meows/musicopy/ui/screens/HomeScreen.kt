package zip.meows.musicopy.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.unit.dp
import musicopy.composeapp.generated.resources.Res
import musicopy.composeapp.generated.resources.content_copy_24px
import org.jetbrains.compose.resources.painterResource
import zip.meows.musicopy.AppSettings
import zip.meows.musicopy.ui.components.DetailBox
import zip.meows.musicopy.ui.components.DetailItem
import zip.meows.musicopy.ui.components.SectionHeader
import zip.meows.musicopy.ui.components.TopBar
import zip.meows.musicopy.ui.widgetHeadline

@Composable
fun HomeScreen(
    onShowNodeStatus: () -> Unit,

    onPickDownloadDirectory: () -> Unit,
    onConnectQRButtonClicked: () -> Unit,
    onConnectManuallyButtonClicked: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopBar(title = "Musicopy", onShowNodeStatus = onShowNodeStatus)
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Box(modifier = Modifier.padding(8.dp)) {
                DetailBox(
                    actionLabel = "Change",
                    onAction = onPickDownloadDirectory,
                ) {
                    val downloadDirectory by AppSettings.downloadDirectoryFlow.collectAsState(
                        null
                    )

                    downloadDirectory?.let { downloadDirectory ->
                        DetailItem("Download Folder", downloadDirectory)
                    }
                }
            }

            HorizontalDivider(thickness = 1.dp)

            HomeSection("CONNECT") {
                Row(
                    modifier = Modifier
                        .fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Button(
                        modifier = Modifier
                            .weight(1f)
                            .height(140.dp),
                        shape = MaterialTheme.shapes.large,
                        onClick = onConnectQRButtonClicked,
                    ) {
                        Column(
                            modifier = Modifier.fillMaxSize(),
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(
                                8.dp,
                                Alignment.CenterVertically
                            )
                        ) {
                            Icon(
                                painter = painterResource(Res.drawable.content_copy_24px),
                                contentDescription = "QR code icon",
                                modifier = Modifier.size(48.dp)
                            )

                            Text("Scan QR code", style = MaterialTheme.typography.bodyLarge)
                        }
                    }

                    Button(
                        modifier = Modifier
                            .weight(1f)
                            .height(140.dp),
                        shape = MaterialTheme.shapes.large,
                        onClick = onConnectManuallyButtonClicked,
                    ) {
                        Column(
                            modifier = Modifier.fillMaxSize(),
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(
                                8.dp,
                                Alignment.CenterVertically
                            )
                        ) {
                            Icon(
                                painter = painterResource(Res.drawable.content_copy_24px),
                                contentDescription = "QR code icon",
                                modifier = Modifier.size(48.dp)
                            )

                            Text("Connect manually", style = MaterialTheme.typography.bodyLarge)
                        }
                    }
                }
            }

            HomeSection("RECENT CONNECTIONS") {
                Text("asdf")
                Text("asdf")
                Text("asdf")
            }
        }
    }
}

@Composable
fun HomeSection(title: String, content: @Composable () -> Unit) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .background(MaterialTheme.colorScheme.surface)
    ) {
        Column {
            SectionHeader(title)

            Box(
                modifier = Modifier
                    .clipToBounds()
            ) {
                Column(
                    modifier = Modifier
                        .padding(8.dp)
                ) {
                    content()
                }
            }
        }
    }
    HorizontalDivider(thickness = 1.dp)
}

