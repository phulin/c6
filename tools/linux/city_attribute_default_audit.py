#!/usr/bin/env python3
"""Static initialized-primary CityAttributes entry0 invariant; no game execution."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA
ROOT = Path(__file__).resolve().parents[2]
METHODS = [[33314752,
  39,
  '__cxx_global_var_init.1',
  '2ed894864d00846f0f9b4371f373cf7fd7a6535f2f2e89945cf80d7857311bdc'],
 [44308528,
  50,
  'GameCore::Map::CityPlots::CityPlots(GameCore::Map::IInstance*)',
  'b0ff7045c8bed11e3d1b0529fa646188f6133298fd83fb37681c8d1665cfe1a1'],
 [44308592,
  23,
  'GameCore::Map::CityPlots::~CityPlots()',
  '0107103d60a7b23d3e8a77bb5582bfd0ba192798263a2dfbefdf7580f51f5392'],
 [44309424,
  116,
  'GameCore::Map::CityPlots::Initialize()',
  '80221f05cf14b77611320579f980794ffa7bdbb5736769cd396f7088c2c4fa04'],
 [44309552,
  97,
  'GameCore::Map::CityPlots::Uninitialize()',
  'c2a5668203856fb12d94cd81d619c7e87a86943995b3e3d6adf9d58d2ac897e0'],
 [44311808,
  50,
  'GameCore::Map::CityPlots::GetAttributes(unsigned int)',
  'd9b066e28f35789167afe32cdf72cfc7287e0678e5b4f1f5324427a3260b862d'],
 [44311872,
  55,
  'GameCore::Map::CityPlots::EditAttributes(unsigned int)',
  '912decf1e43891ad6c0ccc1ce65739d009885b40d8610c364e062a6b7b78abaa'],
 [44311936,
  12,
  'GameCore::Map::CityPlots::AllocateAttributes()',
  '3e4787cfa07a6c5fcd610f59e834b9a6767784c8ad209baf4d5da6b4d85cc93f'],
 [44311952,
  119,
  'GameCore::Map::CityPlots::DeallocateAttributes(unsigned int)',
  '648ba27c8a08d4eddaaf0d9cea3cc27048dd4110ae7c5e856b5ae65b90519942'],
 [45918080,
  12,
  'GameCore::Map::Plot::Instance::GetCityAttributes() const',
  'e1a598f85f49a28e29144ddf86739e2601b0294590320845586249ce08532b8a'],
 [45918096,
  31,
  'GameCore::Map::Plot::Instance::EditCityAttributes()',
  '4a427d1a077cc38c84d74cedd9cf44bb6a417367a8676ed4d99ac9da0a345511'],
 [45918128,
  9,
  'GameCore::Map::Plot::Instance::HasCityAttributes() const',
  '465525d3e6485a0fa870265793fdd0e49577850a4fb46c4514eb2bb42fd126d2'],
 [45918144,
  21,
  'GameCore::Map::Plot::Instance::ClearCityAttributes()',
  '1b9bb592ab5eb45e8a9dba58981ee9a5679f0f78addac4acd5a63689c20f9583'],
 [45918176,
  7,
  'GameCore::Map::Plot::Instance::InitializeCityAttributes()',
  'd3950acc3abbc76fb8096f4da6e53a2b26b493d93923eacc8c600b40105fe736'],
 [45910048,
  1077,
  'GameCore::Map::Plot::Instance::Read(Serialization::IStream&, int)',
  'd8c6bd76add824f31a23a544617d6b211f486a0854537d69ff4ff08eb78f0276'],
 [41662096,
  16,
  'Types::SparseVector<Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes>, '
  'Types::LinearAlloc::Default<1u, 0u> >::SparseVector()',
  'cd92a354427540a2ee849fab1c7d6da6eaf9ab27f4206430d98c9f92ced63508'],
 [41662144,
  28,
  'Types::SparseVector<Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes>, '
  'Types::LinearAlloc::Default<1u, 0u> >::clear()',
  'bc7c33c10e913e81c1b09bd0865b8aac8b28d862b227563dd3c95c287aeefa90'],
 [41662176,
  117,
  'Types::SparseVector<Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes>, '
  'Types::LinearAlloc::Default<1u, 0u> >::Alloc()',
  'f3fb7cc7bde614dbb169f08342e9464002aa30fef0ed5887360c22b24048fc3e'],
 [41662416,
  127,
  'Types::SparseVector<Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes>, '
  'Types::LinearAlloc::Default<1u, 0u> >::Free(unsigned int)',
  'bccdb2e1dc259692be3767248ce53e110246c12d899866659010f6ff83c322ec'],
 [41662336,
  55,
  'Types::SparseVector<Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes>, '
  'Types::LinearAlloc::Default<1u, 0u> >::is_element_valid(unsigned int) const',
  '840bd9baaf98196274b11a0390735fc9b9fffd0c3dfc349bea624239f921b10c'],
 [41662128,
  11,
  'Types::SparseVector<Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes>, '
  'Types::LinearAlloc::Default<1u, 0u> >::Reserve(unsigned int)',
  'fdfeed92c7c5e8afeb0a207ae6ebab5df4aacde66b39d1ee8a76ddd95f130a52'],
 [41662304,
  22,
  'Types::SparseVector<Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes>, '
  'Types::LinearAlloc::Default<1u, 0u> >::destroy()',
  'e454776255a476e21a072c2f709ce949850b8bbe7f32d81dfaebf3be40732c3f'],
 [41663328,
  60,
  'Types::Vector<Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes>, '
  'Types::LinearAlloc::Default<1u, 0u> >::push_back()',
  '2c83a60a4c3e00954cf8119df5eb3721fce986bb98ada6908971b17e95684544'],
 [41662832,
  196,
  'Types::Vector<Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes>, '
  'Types::LinearAlloc::Default<1u, 0u> >::RequestLinearBlock(unsigned int, bool)',
  '34dc36247cf4e5c8f7c79d5c4a83c879197101853ba1114f368706ef6d0124c9'],
 [41663040,
  117,
  'Types::LinearAlloc::LinearBlock<Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes> > '
  'Types::LinearAlloc::DefaultStatic<Platform::StaticHeapAllocator<1, 0> '
  '>::AllocLinearBlock<Types::LinearAlloc::LinearBlock<Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes> '
  '> >(Types::LinearAlloc::LinearBlock<Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes> > '
  'const&, unsigned int, bool)',
  'dbe01b7db95a10158721f6aab62d0d587010fada7b04a821b8fcf099eea5430d'],
 [41663168,
  19,
  'Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes>::SparseVectorNodePolicy(Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes>&&)',
  '69342130dde50f3a80445060cc01a2fcfb7a10613fb55f88b0df064279f9a68f'],
 [41663472,
  32,
  'GameCore::Map::Plot::CityAttributes::CityAttributes()',
  '678fcdd56f78ea2f47ce6b9d44f35e598b66e1072bd4b45a808bec4629268595'],
 [41663456,
  9,
  'Types::SparseVectorNodePolicy<GameCore::Map::Plot::CityAttributes>::SparseVectorNodePolicy()',
  '0d8be594b7d1c4d737dc18714692ed3eb0cc8654ca5fbbdc3026f21952c1af18'],
 [41663520,
  39,
  'GameCore::Map::Plot::CityAttributes::Initialize()',
  '8c63b3a6c242a1a59e3eecf7042bff0a94e03e0ba5d9c508ffe948370bc8799d'],
 [41663200,
  54,
  'GameCore::Map::Plot::CityAttributes::CityAttributes(GameCore::Map::Plot::CityAttributes&&)',
  '23e7cfed9518b1282c1440c0b631bcdcb56a039304fd25f47ec4f0b64967cc94'],
 [37906560,
  54,
  'GameCore::Map::Plot::CityAttributes::CityAttributes(GameCore::Map::Plot::CityAttributes const&)',
  '5498fdd816e88dd17f48fc22f16a1b65679b7126517455133fcbbb57999d3d49'],
 [45908112,
  91,
  'GameCore::Map::Plot::CityAttributes::Read(Serialization::IStream&, int)',
  '4735021680e42fe397e22a42fcde4e37114f3a69045747faae2e7a3e29378f77'],
 [45908208,
  16,
  'GameCore::Map::Plot::CityAttributes::Update(GameCore::Map::Plot::Reflection::CityAttributes1 const&)',
  'fd6b61eee446ebca204402ca257237ea444ee42b87e9f4b2076d46c9e38b4cd5'],
 [45908880,
  23,
  'GameCore::Map::Plot::CityAttributes::SetDomainValid(GameCore::DomainMask, bool)',
  '586928f865bc6c6950507556026a70982e878067b2fc6717804de6f330f7f067'],
 [45908912,
  31,
  'GameCore::Map::Plot::CityAttributes::SetDomainValid(GameCore::DomainTypes, bool)',
  'f3b68f8f4168aa4bb78abd50c3b60e6e5d67002d0203be55681b589623727e35'],
 [44294944,
  5,
  'GameCore::Map::Plot::CityAttributes::SetWonderType(GameCore::BuildingTypes)',
  '2f18e1c666165e82089f97118a2c3a69a4b9a9ff7260abcdcf61cb506a9a9b32'],
 [42394704,
  9,
  'GameCore::City::CManager::EditAttributes(GameCore::Map::Plot::Instance*)',
  'db4a088c0f35c41cd214a6116e372acac82a695ae796abf43ff0875ec5a50b51'],
 [41909904,
  163,
  'GameCore::Cache::Map::Plot::Instance::Initialize(GameCore::Map::Plot::Instance const*, bool)',
  'fc3229d26ef2000462285922069ee849f31716f4e37b11192ce92282432b04ef'],
 [41910720,
  143,
  'GameCore::Cache::Map::Plot::Instance::Update(GameCore::Map::Plot::Instance const*, bool)',
  'edaf3255b4faecb3f97e29d3c3a577aa477222610c6513590e99ee42b977cd87'],
 [41910864,
  132,
  'GameCore::Cache::Map::Plot::Instance::Update(GameCore::Cache::Map::EventStream::PlotEntry const&, bool)',
  '25a963ff39e12b3531c628cb322a2d77ff9189ad21ea23227ebfc07a496ace82'],
 [41913152,
  31,
  'GameCore::Cache::Map::Plot::Instance::EditCityAttributes()',
  '77ad06eb9caceecb1ac7f38a29887691b11855105e6b9e8cab8a1bf71b5d5bcf'],
 [41661936,
  55,
  'GameCore::Cache::Map::CityPlots::EditAttributes(unsigned int)',
  '85bb55515d6a9f91f0ce07550f5a8b61dd1b151de40e1ac0e326f8469495fc6e'],
 [42310400,
  857,
  'GameCore::City::BuildQueue::PostCreateDistrict(GameCore::Definition::District const&, GameCore::PlotCoord '
  'const&, bool)',
  '4d245fa14bf2b34cdfc2758d0f7dffc7c09bf6a1336c22ef2f5dcd50a64475a6'],
 [42355616,
  856,
  'GameCore::City::Buildings::SetPillaged(GameCore::BuildingTypes, bool, bool, GameCore::PlayerTypes)',
  '62b46268a2e1d4e78467bbc9c3d9fa53d0b75152d6744cdbc0ba9e7e6c9d4a31'],
 [42487600,
  424,
  'GameCore::City::Districts::Destroy(GameCore::District::Instance*)',
  '6b6b97422a9d4707593ab40d43ebd89b8f0f522b74a2361bce99c36e25bc43a0'],
 [42490880,
  874,
  'GameCore::City::Districts::SetPillaged(GameCore::District::Instance*, bool, GameCore::PlayerTypes, '
  'GameCore::CityTransferTypes)',
  'd7072e168cbb6d9dc79f787e5063ad3c670dbf49b98a6a8283ead4709071e337'],
 [42705808,
  6354,
  'GameCore::City::Manager::Transfer(GameCore::City::Instance*, GameCore::PlayerTypes, '
  'GameCore::CityTransferTypes)',
  'edaa85326d17dd1bf004d918aa9cc8952ec5649941371dd7eef0a1427304a951'],
 [44286512,
  681,
  'GameCore::Map::Improvement::Builder::SetWonderType(GameCore::Map::Plot::Instance*, '
  'GameCore::BuildingTypes, GameCore::PlayerTypes)',
  '8cadf5935efc28bcf414e00d961acc24cd1855014baad06c16a07f3c35dd1b77'],
 [44309120,
  178,
  'GameCore::Map::CityPlots::SetValidDomains(GameCore::Map::Plot::Instance*)',
  '2a4b78123d7faffdefd2c0e7c1841ff9a72ef95454eb2b4859482ac9e388e74b'],
 [45958528,
  953,
  'GameCore::Rules::Cities::Instance::OnConstructionComplete(GameCore::BuildingTypes, GameCore::PlotCoord, '
  'GameCore::City::Instance*, bool)',
  '244fff3797dd3432e51efeff2220001118aaf8223b67e06c83971e57c127ad7a'],
 [45966688,
  165,
  'GameCore::Rules::Cities::Instance::MarkAdjacentCitiesNavigable(GameCore::Map::Plot::Instance*)',
  '0e55899eac1833b3ebe4a697dfed0ad27de88703e308951e9348b166c557a5ec'],
 [47688352,
  2540,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Game::Instance&)',
  '917d430f7b9c105932f394d62b2b75c83f698437ba89e7ce0ece979dad395873'],
 [44331024,
  555,
  'GameCore::Map::Instance::Read(Serialization::IStream&)',
  '203be99fb7e29951094b8c224aa37d5bec3e1fc7cc0b15ff95cef082fb810261'],
 [44331712,
  110,
  'Serialization::operator>>(Serialization::IStream&, eastl::vector<GameCore::Map::Plot::Instance, '
  'eastl::allocator>&)',
  '9cf942f7ab8e710a2a4c5066470c6146f61776ed8d9d33dbfeffdb12d4afa8c2'],
 [44334080,
  146,
  'GameCore::Map::Instance::Initialize(GameCore::Map::InitializationParameters const&)',
  '9a777a8688a455b8e42ca1f38ddbe97a385a5f2149e5062be797cbe3199580fe'],
 [44334240,
  645,
  'GameCore::Map::Instance::Reset(GameCore::Map::InitializationParameters const&)',
  'e2c155e84fb0cd25e125eddb0bed0c7242416f3f8df819e5d09800f92874d191'],
 [44334912,
  294,
  'GameCore::Map::Instance::InitPlots()',
  '87f8f77037c616b3ff4f3ed54404b26706934ffd9f8afca20b02426ea19f6cde'],
 [44338800,
  149,
  'eastl::vector<GameCore::Map::Plot::Instance, eastl::allocator>::resize(unsigned int)',
  'e4c9d5ca5787787416a4894c685a818e30b46121db13d65e52a6e6e217ab7acd'],
 [44341920,
  501,
  'eastl::vector<GameCore::Map::Plot::Instance, '
  'eastl::allocator>::DoInsertValues(GameCore::Map::Plot::Instance*, unsigned int, '
  'GameCore::Map::Plot::Instance const&)',
  '93be35b5188cfc6984bb26200fe6dd33a87ce9d307334264bdf3780208bbcdb7'],
 [45911152,
  53,
  'GameCore::Map::Plot::Instance::Instance()',
  'e87704dc4307eb2e0205e0d27d308d93f5fb2bd434339b9567fc72d193666ba2'],
 [41909744,
  33,
  'GameCore::Cache::Map::Plot::Instance::Instance()',
  '5f60ed6d484ad38bb48981e99704bc1cf2d54927e0309df057f25f57a08e124a'],
 [45911216,
  97,
  'GameCore::Map::Plot::Instance::Initialize(int, int)',
  '44bf4136d46dce05132a0ac246856a5594783e2c391f3a05d133245f844bfc26']]
# All direct target-internal mutation sites. The DomainTypes overload has no
# incoming target-internal calls; it only forwards its receiver to DomainMask.
TARGETS = {
    0x1f9d860: 'get', 0x2a42500: 'get',
    0x1fa7fd0: 'edit', 0x2a42540: 'edit',
    0x1fa1590: 'allocate', 0x2a42580: 'allocate',
    0x1e1ac20: 'deallocate', 0x2a42590: 'deallocate',
    0x1eeee20: 'domain_mask', 0x2bc8390: 'domain_mask',
    0x2bc83b0: 'domain_type',
    0x1f4c2e0: 'wonder', 0x2a3e320: 'wonder',
    0x1e08f90: 'read', 0x2bc8090: 'read',
    0x1ed3ad0: 'update', 0x2bc80f0: 'update',
    0x1f57be0:'primary_plot_ctor',0x1f6d530:'primary_plot_ctor',0x2bc8c70:'primary_plot_ctor',
    0x1efd1c0:'primary_plot_resize',0x2a48e70:'primary_plot_resize',
    0x1ea20f0:'attrs_construct',0x27bbbf0:'attrs_construct',
    0x1f210d0:'attrs_move',0x27bbae0:'attrs_move',
    0x1f92670:'attrs_copy',0x2426880:'attrs_copy',
    0x1e4d360:'attrs_fill',0x27f8670:'attrs_fill',
    0x1f55340:'plot_copy',0x2a49ca0:'plot_copy',0x1e687a0:'plot_assign',0x2a4a020:'plot_assign',
    0x1eef260:'primary_plot_initialize',0x2bc8cb0:'primary_plot_initialize',
}
EXPECTED_CALLS = {
    'attrs_construct':[0x1fc56e7,0x27bbbe4],
    'attrs_move':[0x27bbace],
    'attrs_copy':[0x238a1c6,0x26f13d8,0x29b83dd],
    'attrs_fill':[0x27bc70e,0x2c928ae],
    'plot_copy':[0x2a49ba0,0x2a49f29,0x2a4a0ee],
    'plot_assign':[0x2a49ffa,0x2a4a0a6,0x2a4a166],
    'get': [0x2bca785], 'edit': [0x2bca7a8],
    'allocate': [0x2bca79b], 'deallocate': [0x2bca7c8],
    'domain_mask': [0x2859c2a,0x2864df0,0x2885062,0x2885cf1,0x2885e66,0x28bb117,
                    0x2a41ab5,0x2a41ad8,0x2a41afb,0x2a41b25,0x2bc83ca,
                    0x2bd479c,0x2bd47c7,0x2bd65dc,0x2d7b0ed,0x2d7b18b,0x2d7b241],
    'domain_type': [], 'wonder': [0x2a3c3b4,0x2a3c42a,0x2a3c449],
    'read': [0x2bc8c41], 'update': [0x27f7f0f,0x27f8239,0x27f82bc],
    'primary_plot_ctor':[0x27f7df4,0x2a48ea4],
    'primary_plot_resize':[0x2a472ec,0x2a47f80],
    'primary_plot_initialize':[0x2a48014,0x2bc8ca0],
}
# Exact mutable receiver producers, manually traced through saved registers and
# caller-saved/callee-saved SysV contracts, then pinned by complete body hashes.
RECEIVERS = {
    0x2859c2a:0x2859c1a,0x2864df0:0x2864ddb,0x2885062:0x2885055,
    0x2885cf1:0x2885ce4,0x2885e66:0x2885e56,0x28bb117:0x28bb107,
    0x2a41ab5:0x2a41a8d,0x2a41ad8:0x2a41a8d,0x2a41afb:0x2a41a8d,0x2a41b25:0x2a41a8d,
    0x2bd479c:0x2bd478c,0x2bd47c7:0x2bd47b7,0x2bd65dc:0x2bd65cc,
    0x2d7b0ed:0x2d7b0dd,0x2d7b18b:0x2d7b17b,0x2d7b241:0x2d7b231,
    0x2bc8c41:0x2bc8c35,
    0x27f7f0f:0x27f7f02,0x27f8239:0x27f8227,0x27f82bc:0x27f82ac,
    0x2a3c3b4:0x2a3c250,0x2a3c42a:0x2a3c250,0x2a3c449:0x2a3c250,
}


def require(test, reason):
    if not test: raise ValueError(reason)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--module', type=Path, default=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    module = Elf(args.module, GAME_SHA)
    for address,size,name,expected in METHODS:
        require(hashlib.sha256(module.read(address,size)).hexdigest() == expected, 'method differs: '+name)
    require(module.read(0x2bca7a0,4)==bytes.fromhex('6689434e'), 'native u32 allocation result narrowing changed')
    inventory_path = ROOT/'evidence/linux/city-attribute-writer-inventory.json'
    require(hashlib.sha256(inventory_path.read_bytes()).hexdigest() ==
            '512b759a8a9bc8f18cb61ab7994b6cf03d53a431110c69eb679df5ed750b76a4',
            'writer inventory differs')
    inventory = json.loads(inventory_path.read_text())
    for method in inventory['functions']:
        require(hashlib.sha256(module.read(int(method['address'],16), method['bytes'])).hexdigest()
                == method['sha256'], 'writer containing method differs: '+method['name'])
    small_stores = []
    process = subprocess.Popen(['objdump','-d','-C','-Mintel','--no-show-raw-insn',str(args.module)],
                               stdout=subprocess.PIPE,text=True)
    for line in process.stdout:
        instruction = re.match(r'\s*([0-9a-f]+):\s*(\w+)\s+(.*)',line)
        if not instruction: continue
        address, operation, operands = instruction.groups()
        if operation in ('cmp','test','push','call','jmp','nop'): continue
        destination = operands.split(',',1)[0]
        match = re.match(r'(BYTE|WORD) PTR \[.*\+0x(14|15|1e|1f)\]$', destination)
        if match: small_stores.append((hex(int(address,16)),match[1].lower()))
    require(process.wait()==0, 'small-field store disassembly failed')
    require(small_stores == [(row['address'],row['width']) for row in inventory['small_field_stores']],
            'full-target small-field store inventory differs')
    nm = subprocess.check_output(['nm','-C','-S',str(args.module)],text=True)
    require('0000000003c05df0 0000000000000020 b GameCore::Map::ms_kCityAttributeCache' in nm,
            'primary cache is not the pinned local BSS object')
    require('0000000003c02de0 0000000000000020 b GameCore::Cache::Map::ms_kCityAttributeCache' in nm,
            'separate cache pool differs')
    calls = {name:[] for name in EXPECTED_CALLS}
    references, instructions = [], {}
    function = None
    process = subprocess.Popen(['objdump','-d','-C','--no-show-raw-insn',str(args.module)],stdout=subprocess.PIPE,text=True)
    for line in process.stdout:
        heading = re.match(r'^([0-9a-f]+) <',line)
        if heading: function = int(heading.group(1),16)
        instruction = re.match(r'\s*([0-9a-f]+):\s*(.*)',line)
        if not instruction: continue
        address,operation = int(instruction.group(1),16),instruction.group(2).strip()
        if address in RECEIVERS.values(): instructions[address] = operation
        branch = re.match(r'(?:call|jmp)\s+([0-9a-f]+)\s',operation)
        if branch and int(branch.group(1),16) in TARGETS:
            calls[TARGETS[int(branch.group(1),16)]].append(address)
        reference = re.search(r'# ([0-9a-f]+) ',operation)
        if reference and 0x3c05df0 <= int(reference.group(1),16) < 0x3c05e10:
            references.append(dict(instruction=address,function=function,target=int(reference.group(1),16)))
    require(process.wait()==0,'disassembly failed')
    require(calls == EXPECTED_CALLS, 'target-internal attribute mutation/access call inventory changed: '+str({key:[hex(x) for x in value] for key,value in calls.items() if value!=EXPECTED_CALLS[key]}))
    expected_functions = {0x1fc57c0:1,0x2a41830:1,0x2a41870:1,0x2a41bb0:1,0x2a41c30:1,
                          0x2a42500:1,0x2a42540:3,0x2a42580:1,0x2a42590:3}
    require(len(references)==13 and all(sum(r['function']==f for r in references)==count
            for f,count in expected_functions.items()),'primary cache storage reference inventory changed')
    for address,operation in instructions.items():
        if address==0x2a3c250: require(operation.startswith('call   1efc4b0 '),'Wonder mutable CManager wrapper differs')
        else: require(re.match(r'call\s+\*0x40\(',operation) is not None,'mutable attribute producer differs')
    require(len(instructions)==len(set(RECEIVERS.values())),'mutable receiver instruction missing')
    for slot,target in [(0x38,0x2bca780),(0x40,0x2bca790),(0x48,0x2bca7b0),(0x80,0x2bca7e0),(0x88,0x2bca7c0)]:
        require(module.relocated_pointer(0x3ae2390+16+slot)==target,'primary Plot virtual provider differs')
    report=dict(schema='c6-linux-city-attribute-default-invariant-v1',game_sha256=GAME_SHA,
        original_execution=False,methods=[dict(address=hex(a),bytes=n,name=name,sha256=d) for a,n,name,d in METHODS],
        primary_pool=dict(address='0x3c05df0',bytes=32,local_bss=True,references=references),
        separate_cache_pool='0x3c02de0',
        writer_inventory=dict(path=str(inventory_path.relative_to(ROOT)),sha256=hashlib.sha256(inventory_path.read_bytes()).hexdigest(),small_field_stores=len(small_stores),containing_methods=len(inventory['functions'])),
        calls={name:[hex(a) for a in addresses] for name,addresses in calls.items()},
        mutation_receivers=[dict(write=hex(write),producer=hex(producer),
            source='CManager.EditAttributes -> primary Plot mutable+0x40' if producer==0x2a3c250 else
                   'separate cache Plot mutable+0x40' if write in EXPECTED_CALLS['update'] else 'primary Plot mutable+0x40')
            for write,producer in RECEIVERS.items()],
        invariant=dict(phase='initialized live primary CityPlots; ordinary target-internal mutation paths',
            reserve='Initialize clears sparse vector then Alloc reserves index0',
            initialization='domain i16=0; packed Wonder i16=-1; owner=-1; working/owning/District UINT_MAX; yieldEffect0',
            release='DeallocateAttributes explicitly refuses index0; valid frees therefore never add0 to free chain',
            edit='only direct EditAttributes caller is Plot.EditCityAttributes, which allocates when plot index0',
            relocation='node/vector construction, move/copy and allocation bodies preserve reserved fields',
            read='Plot.Read resets index0 then mutably allocates only for serialized HasCityAttributes',
            reflection='all3 CityAttributes.Update callers are separate Cache::Map Plot updates',
            mask='all16 primary domain-mask writes use mutable owned attributes; nonzero only under the explicit no-wrapped-allocation bound',
            wonder='all3 SetWonderType writes use CManager mutable Plot attributes',
            lifetime='Uninitialize/destruction clear storage; no getter semantics claimed outside initialized live scope'),
        fresh_load_bound=dict(primary_plots='only vector resize fill temporary and Cache derived-base constructor call the primary constructor; cache overrides its vtable/pool before edits',
            reset='Map.Read -> Initialize(params) -> Reset/InitPlots -> CityPlots.Initialize clears/reserves0 before component/Map-initialized callbacks',
            plot_read='one indexed pass of current Plot vector; each resets index0 then allocates at most once for its saved HasCityAttributes',
            orphan_accounting='pre-read canonical plots can already own up to P attributes; Plot.Read can orphan those, so P alone is not the allocation bound',
            bound='for the fresh canonical single-Map read/restore sequence: at most P pre-read orphan slots plus P current owned slots plus reserved0; logical constructed-node high-water <=2*P+1 (not raw allocation capacity)',
            conditions=['positive width and height; P>0', 'canonical primary Plot ownership', 'single current Map.Read pass after reset', 'no extra out-of-band attribute/index mutation', '2*P <=65535'],
            current_fixture_plots=6996,current_fixture_max_allocated_index=13992),
        original_overflow_counterexample=dict(allocation_result_u32=65536,stored_plot_index_u16=0,
            subsequent_edit='EditAttributes(0) sees reserved index0 valid and can return it for mutation',
            source_model_policy='explicit unsupported bound; original overflow and alias behavior remain in full-engine scope'),
        native_default_results_used_as_inputs=False,all_plot_parity_claimed=False,
        full_gameplay_state_actions_turns=False)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print('verified',len(METHODS),'base whole methods,81 extended writer/copy method hashes,92 small-field sites and13 primary-pool references')

if __name__=='__main__': main()
