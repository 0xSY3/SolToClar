;; Contract: Counter
;; Auto-generated Clarity contract from Solidity source

;; @desc Stores the count value
(define-data-var count uint u0)

;; Function: increment
;; @desc Increments the counter by 1
;; @returns (response bool uint) Returns true on success
(define-public (increment)
  (var-set count (+ (var-get count) u1))
  (ok true))

;; Function: getCount
;; @desc Gets the current counter value
;; @returns (response uint uint) Returns the current count
(define-public (getCount)
  (ok (var-get count)))

