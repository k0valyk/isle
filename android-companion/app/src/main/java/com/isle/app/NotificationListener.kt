package com.isle.app

import android.app.Notification
import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors

class NotificationListener : NotificationListenerService() {
    private val executor = Executors.newSingleThreadExecutor()
    private val recent = ConcurrentHashMap<String, Long>()

    override fun onNotificationPosted(sbn: StatusBarNotification) {
        if (sbn.isOngoing) return
        val prefs = getSharedPreferences("isle", MODE_PRIVATE)
        val host = prefs.getString("host", null) ?: return
        val token = prefs.getString("token", null) ?: return

        val extras = sbn.notification.extras
        val title = extras.getCharSequence(Notification.EXTRA_TITLE)?.toString().orEmpty()
        val text = extras.getCharSequence(Notification.EXTRA_TEXT)?.toString().orEmpty()
        if (title.isBlank() && text.isBlank()) return

        // Some apps re-post the same notification several times per second.
        val key = "${sbn.packageName}|$title|$text"
        val now = System.currentTimeMillis()
        val last = recent[key] ?: 0L
        if (now - last < 4000) return
        recent[key] = now
        if (recent.size > 200) {
            recent.entries.removeIf { now - it.value > 60_000 }
        }

        val app = try {
            val info = packageManager.getApplicationInfo(sbn.packageName, 0)
            packageManager.getApplicationLabel(info).toString()
        } catch (_: Exception) {
            sbn.packageName
        }

        executor.execute {
            try {
                val payload = JSONObject()
                    .put("app", app)
                    .put("title", title)
                    .put("text", text)
                    .toString()
                val connection = (URL("http://$host/notification").openConnection() as HttpURLConnection).apply {
                    requestMethod = "POST"
                    connectTimeout = 5000
                    readTimeout = 8000
                    doOutput = true
                    setRequestProperty("Content-Type", "application/json")
                    setRequestProperty("X-Isle-Token", token)
                }
                connection.outputStream.use { it.write(payload.toByteArray()) }
                connection.responseCode
                connection.disconnect()
            } catch (_: Exception) {
                // The PC may be offline; the next notification retries.
            }
        }
    }

    override fun onDestroy() {
        executor.shutdown()
        super.onDestroy()
    }
}
