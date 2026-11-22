import StoreKit

// StoreKit 2 has no C or Objective-C interface, and only it tells without a question to the person what the
// Apple Account owns. The Rust binary loads this framework and calls the one function.
@_cdecl("fractique_pro_watch")
public func fractiqueProWatch(_ product: UnsafePointer<CChar>, _ tell: @escaping @convention(c) (Bool) -> Void) {
    let wanted = String(cString: product)
    Task.detached {
        for await entitlement in Transaction.currentEntitlements {
            if case .verified(let bought) = entitlement, bought.productID == wanted, bought.revocationDate == nil {
                tell(true)
            }
        }
        for await update in Transaction.updates {
            if case .verified(let bought) = update, bought.productID == wanted {
                tell(bought.revocationDate == nil)
            }
        }
    }
}
