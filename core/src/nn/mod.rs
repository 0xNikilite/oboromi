use crate::nn;
use crate::sys;

macro_rules! define_service {
    ($($name:ident),* $(,)?) => {
        $(
            pub mod $name {
                use crate::nn::ServiceTrait;
                use crate::sys;
                
                pub struct State {}
                
                impl State {
                    pub fn new(_state: &mut sys::State) -> Self {
                        Self {}
                    }
                }
                
                 impl ServiceTrait for State {
                     fn run(state: &mut sys::State) {
                         state.services.$name = Some(State::new(state));
                     }
                 }
             }
         )*
     };
 }

define_service!(
    acc, adraw, ahid, aoc, apm, applet_ae, applet_oe, arp,
    aud, audctl, auddebug, auddev, auddmg, audin, audout,
    audrec, audren, audsmx, avm, banana, batlog, bcat, bgtc,
    bpc, bpmpmr, bsd, bsdcfg, bt, btdrv, btm, btp, capmtp,
    caps, caps2, cec_mgr, chat, clkrst, codecctl, csrng,
    dauth, disp, dispdrv, dmnt, dns, dt, ectx, erpt, es,
    eth, ethc, eupld, fan, fatal, fgm, file_io, friend, fs,
    fsp_ldr, fsp_pr, fsp_srv, gds, gpio, gpuk, grc, gsv,
    hdcp, hid, hidbus, host1x, hshl, htc, htcs, hwopus, i2c,
    idle, ifcfg, imf, ins, irs, jit, lbl, ldn, ldr, led, lm,
    lp2p, lr, manu, mig, mii, miiimg, mm, mnpp, ncm, nd, ndd,
    ndrm, news, nfc, nfp, ngc, ngct, nifm, nim, notif, npns,
    ns, nsd, ntc, nvdbg, nvdrv, nvdrvdbg, nvgem, nvmemp,
    olsc, omm, ommdisp, ovln, pcie, pcm, pctl, pcv, pdm,
    pgl, pinmux, pl, pm, prepo, psc, psm, pwm, rgltr, ro,
    rtc, sasbus, set, sf_uds, sfdnsres, spbg, spi, spl,
    sprof, spsm, srepo, ssl, syncpt, tc, tcap, time,
    tma_log, tmagent, ts, tspm, uart, usb, vi, vi2, vic,
    wlan, xcd,
);

pub trait ServiceTrait {
    fn run(_state: &mut sys::State) -> () {
        todo!();
    }
}

pub fn start_host_services(state: &mut sys::State) {
        let entries: [(&str, fn(&mut sys::State)); 160] = [
            ("acc", nn::acc::State::run),
            ("adraw", nn::adraw::State::run),
            ("ahid", nn::ahid::State::run),
            ("aoc", nn::aoc::State::run),
            ("apm", nn::apm::State::run),
            ("applet_ae", nn::applet_ae::State::run),
            ("applet_oe", nn::applet_oe::State::run),
            ("arp", nn::arp::State::run),
            ("aud", nn::aud::State::run),
            ("audctl", nn::audctl::State::run),
            ("auddebug", nn::auddebug::State::run),
            ("auddev", nn::auddev::State::run),
            ("auddmg", nn::auddmg::State::run),
            ("audin", nn::audin::State::run),
            ("audout", nn::audout::State::run),
            ("audrec", nn::audrec::State::run),
            ("audren", nn::audren::State::run),
            ("audsmx", nn::audsmx::State::run),
            ("avm", nn::avm::State::run),
            ("banana", nn::banana::State::run),
            ("batlog", nn::batlog::State::run),
            ("bcat", nn::bcat::State::run),
            ("bgtc", nn::bgtc::State::run),
            ("bpc", nn::bpc::State::run),
            ("bpmpmr", nn::bpmpmr::State::run),
            ("bsd", nn::bsd::State::run),
            ("bsdcfg", nn::bsdcfg::State::run),
            ("bt", nn::bt::State::run),
            ("btdrv", nn::btdrv::State::run),
            ("btm", nn::btm::State::run),
            ("btp", nn::btp::State::run),
            ("capmtp", nn::capmtp::State::run),
            ("caps", nn::caps::State::run),
            ("caps2", nn::caps2::State::run),
            ("cec_mgr", nn::cec_mgr::State::run),
            ("chat", nn::chat::State::run),
            ("clkrst", nn::clkrst::State::run),
            ("codecctl", nn::codecctl::State::run),
            ("csrng", nn::csrng::State::run),
            ("dauth", nn::dauth::State::run),
            ("disp", nn::disp::State::run),
            ("dispdrv", nn::dispdrv::State::run),
            ("dmnt", nn::dmnt::State::run),
            ("dns", nn::dns::State::run),
            ("dt", nn::dt::State::run),
            ("ectx", nn::ectx::State::run),
            ("erpt", nn::erpt::State::run),
            ("es", nn::es::State::run),
            ("eth", nn::eth::State::run),
            ("ethc", nn::ethc::State::run),
            ("eupld", nn::eupld::State::run),
            ("fan", nn::fan::State::run),
            ("fatal", nn::fatal::State::run),
            ("fgm", nn::fgm::State::run),
            ("file_io", nn::file_io::State::run),
            ("friend", nn::friend::State::run),
            ("fs", nn::fs::State::run),
            ("fsp_ldr", nn::fsp_ldr::State::run),
            ("fsp_pr", nn::fsp_pr::State::run),
            ("fsp_srv", nn::fsp_srv::State::run),
            ("gds", nn::gds::State::run),
            ("gpio", nn::gpio::State::run),
            ("gpuk", nn::gpuk::State::run),
            ("grc", nn::grc::State::run),
            ("gsv", nn::gsv::State::run),
            ("hdcp", nn::hdcp::State::run),
            ("hid", nn::hid::State::run),
            ("hidbus", nn::hidbus::State::run),
            ("host1x", nn::host1x::State::run),
            ("hshl", nn::hshl::State::run),
            ("htc", nn::htc::State::run),
            ("htcs", nn::htcs::State::run),
            ("hwopus", nn::hwopus::State::run),
            ("i2c", nn::i2c::State::run),
            ("idle", nn::idle::State::run),
            ("ifcfg", nn::ifcfg::State::run),
            ("imf", nn::imf::State::run),
            ("ins", nn::ins::State::run),
            ("irs", nn::irs::State::run),
            ("jit", nn::jit::State::run),
            ("lbl", nn::lbl::State::run),
            ("ldn", nn::ldn::State::run),
            ("ldr", nn::ldr::State::run),
            ("led", nn::led::State::run),
            ("lm", nn::lm::State::run),
            ("lp2p", nn::lp2p::State::run),
            ("lr", nn::lr::State::run),
            ("manu", nn::manu::State::run),
            ("mig", nn::mig::State::run),
            ("mii", nn::mii::State::run),
            ("miiimg", nn::miiimg::State::run),
            ("mm", nn::mm::State::run),
            ("mnpp", nn::mnpp::State::run),
            ("ncm", nn::ncm::State::run),
            ("nd", nn::nd::State::run),
            ("ndd", nn::ndd::State::run),
            ("ndrm", nn::ndrm::State::run),
            ("news", nn::news::State::run),
            ("nfc", nn::nfc::State::run),
            ("nfp", nn::nfp::State::run),
            ("ngc", nn::ngc::State::run),
            ("ngct", nn::ngct::State::run),
            ("nifm", nn::nifm::State::run),
            ("nim", nn::nim::State::run),
            ("notif", nn::notif::State::run),
            ("npns", nn::npns::State::run),
            ("ns", nn::ns::State::run),
            ("nsd", nn::nsd::State::run),
            ("ntc", nn::ntc::State::run),
            ("nvdbg", nn::nvdbg::State::run),
            ("nvdrv", nn::nvdrv::State::run),
            ("nvdrvdbg", nn::nvdrvdbg::State::run),
            ("nvgem", nn::nvgem::State::run),
            ("nvmemp", nn::nvmemp::State::run),
            ("olsc", nn::olsc::State::run),
            ("omm", nn::omm::State::run),
            ("ommdisp", nn::ommdisp::State::run),
            ("ovln", nn::ovln::State::run),
            ("pcie", nn::pcie::State::run),
            ("pcm", nn::pcm::State::run),
            ("pctl", nn::pctl::State::run),
            ("pcv", nn::pcv::State::run),
            ("pdm", nn::pdm::State::run),
            ("pgl", nn::pgl::State::run),
            ("pinmux", nn::pinmux::State::run),
            ("pl", nn::pl::State::run),
            ("pm", nn::pm::State::run),
            ("prepo", nn::prepo::State::run),
            ("psc", nn::psc::State::run),
            ("psm", nn::psm::State::run),
            ("pwm", nn::pwm::State::run),
            ("rgltr", nn::rgltr::State::run),
            ("ro", nn::ro::State::run),
            ("rtc", nn::rtc::State::run),
            ("sasbus", nn::sasbus::State::run),
            ("set", nn::set::State::run),
            ("sf_uds", nn::sf_uds::State::run),
            ("sfdnsres", nn::sfdnsres::State::run),
            ("spbg", nn::spbg::State::run),
            ("spi", nn::spi::State::run),
            ("spl", nn::spl::State::run),
            ("sprof", nn::sprof::State::run),
            ("spsm", nn::spsm::State::run),
            ("srepo", nn::srepo::State::run),
            ("ssl", nn::ssl::State::run),
            ("syncpt", nn::syncpt::State::run),
            ("tc", nn::tc::State::run),
            ("tcap", nn::tcap::State::run),
            ("time", nn::time::State::run),
            ("tma_log", nn::tma_log::State::run),
            ("tmagent", nn::tmagent::State::run),
            ("ts", nn::ts::State::run),
            ("tspm", nn::tspm::State::run),
            ("uart", nn::uart::State::run),
            ("usb", nn::usb::State::run),
            ("vi", nn::vi::State::run),
            ("vi2", nn::vi2::State::run),
            ("vic", nn::vic::State::run),
            ("wlan", nn::wlan::State::run),
            ("xcd", nn::xcd::State::run),
        ];
        for (_name, run_fn) in entries.iter() {
            run_fn(state);
        }
    }
