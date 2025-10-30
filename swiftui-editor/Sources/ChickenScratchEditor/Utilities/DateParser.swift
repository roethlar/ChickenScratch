import Foundation

enum DateParser {
    private static let isoFormatter: ISO8601DateFormatter = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter
    }()

    private static let fallbackFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd HH:mm:ss ZZZZZ"
        return formatter
    }()

    static func parse(_ value: String) -> Date {
        if let date = isoFormatter.date(from: value) {
            return date
        }
        if let date = fallbackFormatter.date(from: value) {
            return date
        }
        return Date()
    }

    static func encode(_ date: Date) -> String {
        isoFormatter.string(from: date)
    }
}
