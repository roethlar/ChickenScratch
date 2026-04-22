import SwiftUI

extension View {
    /// Rounded-rectangle Liquid Glass panel. Wrap call sites in a `GlassEffectContainer`
    /// when multiple glass elements are stacked in the same layout.
    func panelGlass(cornerRadius: CGFloat = 20) -> some View {
        self.glassEffect(.regular, in: .rect(cornerRadius: cornerRadius))
    }

    /// Capsule Liquid Glass — for small chips and floating controls.
    func capsuleGlass() -> some View {
        self.glassEffect(.regular, in: .capsule)
    }
}
