/* hotfix3 B-7-equivalent: scroll + IntersectionObserver, no i18n */
(function () {
  var nav = document.getElementById('nav');
  window.addEventListener('scroll', function () {
    nav.classList.toggle('scrolled', window.scrollY > 8);
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

  if (!('IntersectionObserver' in window)) {
    document.querySelectorAll('.reveal').forEach(function (e) { e.classList.add('in'); });
    return;
  }
  var io = new IntersectionObserver(function (entries) {
    entries.forEach(function (e) {
      if (e.isIntersecting) { e.target.classList.add('in'); io.unobserve(e.target); }
    });
  }, { threshold: 0.12, rootMargin: '0px 0px -30px 0px' });
  document.querySelectorAll('.reveal').forEach(function (e) { io.observe(e); });
})();
