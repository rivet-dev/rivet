var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBadminton.js
var require_faBadminton = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBadminton.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "badminton";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e33a";
    var svgPathData = "M349 64c-36 0-76.4 15.8-108.8 48.2S192 184.9 192 221c0 28.2 9.6 53.2 27.7 71.3c40.4 40.4 120.7 38.9 180.1-20.5C432.2 239.4 448 199.1 448 163c0-28.2-9.6-53.2-27.7-71.3S377.1 64 349 64zm-154 2.9C238.5 23.4 294.8 0 349 0c42.8 0 84.9 14.8 116.6 46.5S512 120.2 512 163c0 54.2-23.4 110.5-66.9 154c-54.4 54.4-131.9 78.7-198.2 61.7c-29.4-7.5-62.9-5.5-84.3 16L148.3 409c6.4 12.1 4.5 27.4-5.6 37.6l-56 56c-12.5 12.5-32.8 12.5-45.3 0l-32-32c-12.5-12.5-12.5-32.8 0-45.3l56-56c10.2-10.2 25.5-12.1 37.6-5.6l14.5-14.5c21.4-21.4 23.5-54.7 16-84C129.8 250.9 128 236 128 221c0-54.2 23.4-110.5 66.9-154zM464 400c0-26.5 21.5-48 48-48s48 21.5 48 48l0 32-96 0 0-32zm-15.3 91.4l13-43.4 33.4 0-15.8 52.6c-2.5 8.5-11.5 13.3-19.9 10.7s-13.3-11.5-10.7-19.9zM496 496l0-48 32 0 0 48c0 8.8-7.2 16-16 16s-16-7.2-16-16zm48.7 4.6L528.9 448l33.4 0 13 43.4c2.5 8.5-2.3 17.4-10.7 19.9s-17.4-2.3-19.9-10.7z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faBadminton = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faBadminton.js
var import_faBadminton = __toESM(require_faBadminton());
var export_faBadminton = import_faBadminton.definition;
export {
  export_faBadminton as faBadminton
};
