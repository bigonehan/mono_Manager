(() => {
  const ctaButton = document.getElementById('cta-button');
  const ctaStatus = document.getElementById('cta-status');

  if (!ctaButton || !ctaStatus) {
    return;
  }

  const setMessage = () => {
    ctaStatus.textContent = '상담 신청이 접수되었습니다. 1영업일 내 연락드리겠습니다.';
  };

  ctaButton.addEventListener('click', setMessage);
})();
