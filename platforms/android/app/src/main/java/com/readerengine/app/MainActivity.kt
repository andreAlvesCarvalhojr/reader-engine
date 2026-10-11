package com.readerengine.app

import android.net.Uri
import android.os.Bundle
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import uniffi.reader_ffi.FfiReaderSession
import uniffi.reader_ffi.ReaderEngine
import uniffi.reader_ffi.ReaderSnapshot
import androidx.compose.ui.Modifier as ComposeModifier

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val engine = ReaderEngine()
        setContent {
            MaterialTheme(
                colorScheme = darkColorScheme(
                    primary = Color(0xFF3ECFAC),
                    onPrimary = Color(0xFF04201A),
                    background = Color(0xFF071512),
                    surface = Color(0xFF0C2420),
                    onBackground = Color(0xFFD7EFE7),
                    onSurface = Color(0xFFD7EFE7),
                ),
            ) {
                Surface(modifier = ComposeModifier.fillMaxSize()) {
                    ReaderScreen(
                        engine = engine,
                        readAsset = { path -> assets.open(path).use { it.readBytes() } },
                        readUri = { uri ->
                            contentResolver.openInputStream(uri)?.use { it.readBytes() }
                                ?: error("nao foi possivel ler $uri")
                        },
                        displayName = { uri ->
                            uri.lastPathSegment ?: "book.bin"
                        },
                    )
                }
            }
        }
    }
}

private data class SampleBook(
    val id: String,
    val title: String,
    val assetPath: String,
    val fileName: String,
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ReaderScreen(
    engine: ReaderEngine,
    readAsset: (String) -> ByteArray,
    readUri: (Uri) -> ByteArray,
    displayName: (Uri) -> String,
) {
    val samples = remember {
        listOf(
            SampleBook("noite-clara", "Noite Clara", "samples/noite-clara.json", "noite-clara.json"),
            SampleBook("epub-fixture", "EPUB Fixture", "samples/sample.epub", "sample.epub"),
        )
    }

    var session by remember { mutableStateOf<FfiReaderSession?>(null) }
    var snapshot by remember { mutableStateOf<ReaderSnapshot?>(null) }
    var html by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var fontScale by remember { mutableFloatStateOf(1f) }
    var activeSample by remember { mutableStateOf<String?>(null) }

    fun refresh(from: FfiReaderSession, snap: ReaderSnapshot) {
        session = from
        snapshot = snap
        fontScale = snap.fontScale
        html = from.currentHtml()
        error = null
    }

    fun openBytes(name: String, bytes: ByteArray, sampleId: String?) {
        runCatching {
            val opened = engine.open(name, bytes)
            refresh(opened, opened.snapshot())
            activeSample = sampleId
        }.onFailure {
            error = it.message ?: it.toString()
        }
    }

    val picker = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        runCatching {
            openBytes(displayName(uri), readUri(uri), sampleId = null)
        }.onFailure {
            error = it.message ?: it.toString()
        }
    }

    LaunchedEffect(Unit) {
        val first = samples.first()
        openBytes(first.fileName, readAsset(first.assetPath), first.id)
    }

    Scaffold(
        modifier = ComposeModifier
            .fillMaxSize()
            .statusBarsPadding()
            .navigationBarsPadding(),
        bottomBar = {
            val snap = snapshot
            val index = snap?.spineIndex?.toInt() ?: 0
            val total = snap?.spineLen?.toInt() ?: 0
            Row(
                modifier = ComposeModifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 12.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OutlinedButton(
                    onClick = { session?.let { refresh(it, it.prev()) } },
                    enabled = snap != null && index > 0,
                    modifier = ComposeModifier.height(48.dp),
                ) {
                    Text("Anterior")
                }
                Text(
                    text = if (snap == null) "-" else "${index + 1} / $total",
                    modifier = ComposeModifier.weight(1f),
                    style = MaterialTheme.typography.bodyMedium,
                )
                Button(
                    onClick = { session?.let { refresh(it, it.next()) } },
                    enabled = snap != null && index + 1 < total,
                    modifier = ComposeModifier.height(48.dp),
                ) {
                    Text("Proxima")
                }
            }
        },
    ) { padding ->
        Column(
            modifier = ComposeModifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 16.dp),
        ) {
            Text(
                text = "reader-engine",
                style = MaterialTheme.typography.headlineLarge.copy(fontWeight = FontWeight.Bold),
            )
            Text(
                text = snapshot?.let { "${it.title} · ${it.authors.joinToString()}" }
                    ?: "Abra um arquivo .epub / .json ou use uma amostra",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.75f),
            )
            Spacer(modifier = ComposeModifier.height(12.dp))
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = ComposeModifier.fillMaxWidth(),
            ) {
                samples.forEach { sample ->
                    FilterChip(
                        selected = activeSample == sample.id,
                        onClick = {
                            openBytes(sample.fileName, readAsset(sample.assetPath), sample.id)
                        },
                        label = { Text(sample.title) },
                    )
                }
                OutlinedButton(
                    onClick = {
                        picker.launch(
                            arrayOf(
                                "application/epub+zip",
                                "application/json",
                                "application/octet-stream",
                                "*/*",
                            ),
                        )
                    },
                    modifier = ComposeModifier.height(40.dp),
                ) {
                    Text("Abrir arquivo")
                }
            }
            Spacer(modifier = ComposeModifier.height(8.dp))
            Text("Texto", style = MaterialTheme.typography.labelMedium)
            Slider(
                value = fontScale,
                onValueChange = { fontScale = it },
                onValueChangeFinished = {
                    session?.let { refresh(it, it.setFontScale(fontScale)) }
                },
                valueRange = 0.8f..1.8f,
            )
            error?.let {
                Text(it, color = MaterialTheme.colorScheme.error)
                Spacer(modifier = ComposeModifier.height(8.dp))
            }
            AndroidView(
                modifier = ComposeModifier
                    .fillMaxWidth()
                    .weight(1f),
                factory = { context ->
                    WebView(context).apply {
                        webViewClient = WebViewClient()
                        settings.javaScriptEnabled = false
                        setBackgroundColor(0xFFF3FAF7.toInt())
                    }
                },
                update = { webView ->
                    webView.loadDataWithBaseURL(
                        null,
                        html.ifBlank { "<html><body></body></html>" },
                        "text/html",
                        "utf-8",
                        null,
                    )
                },
            )
        }
    }
}
