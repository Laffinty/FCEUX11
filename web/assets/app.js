/* FCEUX11 landing: scroll + IntersectionObserver + progress bar + stagger, no i18n */
(function () {
  var reduce = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  var nav = document.getElementById('nav');

  // Scroll progress bar (top edge)
  var progress = document.getElementById('progress');
  var progressBar = progress ? progress.querySelector('i') : null;

  window.addEventListener('scroll', function () {
    nav.classList.toggle('scrolled', window.scrollY > 8);
    if (progressBar) {
      var h = document.documentElement.scrollHeight - window.innerHeight;
      var p = h > 0 ? (window.scrollY / h) * 100 : 0;
      progressBar.style.width = p.toFixed(2) + '%';
    }
  }, { passive: true });

  // Language dropdown toggle
  var langDd = document.getElementById('langDd');
  var langBtn = document.getElementById('langBtn');
  if (langDd && langBtn) {
    langBtn.addEventListener('click', function (e) {
      e.stopPropagation();
      langDd.classList.toggle('open');
      langBtn.setAttribute('aria-expanded', langDd.classList.contains('open'));
    });
    document.addEventListener('click', function () {
      langDd.classList.remove('open');
      langBtn.setAttribute('aria-expanded', 'false');
    });
    langDd.addEventListener('click', function (e) {
      e.stopPropagation();
    });
  }

  // Reveal-on-scroll with stagger; fallback for old browsers
  if (!('IntersectionObserver' in window)) {
    document.querySelectorAll('.reveal').forEach(function (e) { e.classList.add('in'); });
    return;
  }
  var io = new IntersectionObserver(function (entries) {
    entries.forEach(function (e) {
      if (e.isIntersecting) {
        e.target.classList.add('in');
        io.unobserve(e.target);
      }
    });
  }, { threshold: 0.12, rootMargin: '0px 0px -30px 0px' });
  document.querySelectorAll('.reveal').forEach(function (e) {
    // Stagger siblings only when motion is allowed
    if (!reduce) {
      var parent = e.parentElement;
      if (parent) {
        var idx = Array.prototype.indexOf.call(parent.children, e);
        e.style.transitionDelay = Math.min(idx * 90, 450) + 'ms';
      }
    }
    io.observe(e);
  });
})();
