use libp2p::pnet::PreSharedKey;

const KEY: [u8; 32] = [
    165, 192, 223, 76, 228, 230, 173, 211, 142, 172, 181, 95, 163, 103, 52, 40, 245, 84, 206, 171,
    66, 60, 198, 86, 93, 253, 55, 181, 53, 210, 209, 87,
];

pub fn key() -> PreSharedKey {
    PreSharedKey::new(KEY)
}
