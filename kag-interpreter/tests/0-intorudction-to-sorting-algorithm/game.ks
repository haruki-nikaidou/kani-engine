*start
#Senpai
Hey, welcome to today's session on sorting algorithms!
@l
Before we dive in — are you already familiar with computational complexity?
@l
@link target=*knows_complexity
Yes, I am!
@link target=*no_complexity
No, not really.
@endlink

; ── Branch: player does not know complexity ──────────────────────────────────

*no_complexity
#Senpai
No worries at all! Let me give you a quick introduction.
@l
[slide name=big_o_intro]
Computational complexity describes how an algorithm's runtime or memory usage grows as the input size increases.
@l
The most common way to express this is Big O notation. O(1) is constant time, O(n) is linear, O(n log n) is linearithmic, and O(n²) is quadratic — the bigger the exponent, the worse it scales.
@l
[slide name=big_o_intro action=hide]
#Me
Ah, so Big O tells us exactly how badly things slow down as data gets bigger. That actually makes a lot of sense!
@l
@jump target=*insertion_sort

; ── Step 4 — Insertion sort ──────────────────────────────────────────────────
; Reached directly when the player already knows complexity.

*knows_complexity
*insertion_sort
#Senpai
Great! Let's start with the classic: insertion sort.
@l
Imagine you're sorting a hand of playing cards. You pick one card at a time and slide it into its correct position among the already-sorted cards in your hand. Simple and intuitive.
@l
It runs in O(n²) in the worst case, so it's fine for small datasets but gets painfully slow on large ones.
@l
#Me
That makes perfect sense. Does the computer actually sort like that?
@l

; ── Step 5 — Choose a sorting algorithm ──────────────────────────────────────

*choose_algorithm
#Senpai
Excellent question! Computers have many algorithms to pick from, each with its own trade-offs. Which kind interests you most?
@l
@link target=*merge_sort
A stable one.
@link target=*quick_sort
A slightly fast one.
@link target=*bogo_sort
A crazy one.
@endlink

; ── Step 6 — Merge sort ──────────────────────────────────────────────────────

*merge_sort
#Senpai
Great taste — merge sort!
@l
It's a divide-and-conquer algorithm: split the array in half, recursively sort each half, then merge the two sorted halves back together.
@l
Time complexity is O(n log n) in all cases, and crucially it is stable — equal elements always keep their original relative order. Very reliable.
@l
#Me
Split, conquer, merge. That's genuinely elegant.
@l
@jump target=*rust_question

; ── Step 7 — Quicksort ───────────────────────────────────────────────────────

*quick_sort
#Senpai
Bold choice — quicksort!
@l
You pick a pivot element, partition the array so smaller elements go left and larger ones go right, then recurse on each partition. No extra memory needed.
@l
Average case is O(n log n) with a tiny constant factor — often the fastest sorting algorithm in practice. However it is not stable, and the worst case degrades to O(n²).
@l
#Me
Faster on average but not stable. I can already see the trade-off!
@l
@jump target=*rust_question

; ── Step 8 — Bogo sort (loops back to step 5) ────────────────────────────────

*bogo_sort
#Senpai
You're daring — bogo sort, the algorithm of pure chaos!
@l
The idea: shuffle the array randomly and check whether it happens to be sorted. If not, shuffle again. Repeat until you get lucky.
@l
Average case is O(n × n!). Worst case is theoretically infinite. Completely impractical — but technically correct.
@l
#Me
That's absolutely absurd! Why does this even exist as an algorithm?!
@l
#Senpai
The joke ends here. Let's get back to learning.
@l
@jump target=*choose_algorithm

; ── Step 9 — Which algorithm does Rust use? ──────────────────────────────────

*rust_question
#Me
Specifically, which sorting algorithm does Rust use in its standard library?
@l
#Senpai
Voultapher/driftsort.
@l
It is a state-of-the-art hybrid that combines the stability of merge sort with the cache-efficiency of pattern-defeating quicksort. It delivers excellent real-world performance across a wide range of input patterns.
@l
#Me
So Rust uses cutting-edge sorting research — that's really cool!
@l