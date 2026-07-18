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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGunSlash.js
var require_faGunSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGunSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "gun-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e19c";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7L412.3 297.8 432.9 240l31.1 0c8.5 0 16.6-3.4 22.6-9.4L509.3 208l66.7 0c17.7 0 32-14.3 32-32l0-80c0-17.7-14.3-32-32-32l-16 0 0-8c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 8L113.9 64 38.8 5.1zM195.6 128L496 128c8.8 0 16 7.2 16 16s-7.2 16-16 16l-259.6 0-40.8-32zM338.5 240l43.4 0-9.5 26.6L338.5 240zM324.9 352L264 304l-3 0 .5-2L32 121.2 32 208c0 17.7 14.3 32 32 32l10 0c20.8 0 36.1 19.6 31 39.8L65 440.2c-2.4 9.6-.2 19.7 5.8 27.5S86.1 480 96 480l96 0c14.7 0 27.5-10 31-24.2L249 352l75.9 0z";
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
    exports.faGunSlash = exports.definition;
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

// src/_entries/faGunSlash.js
var import_faGunSlash = __toESM(require_faGunSlash());
var export_faGunSlash = import_faGunSlash.definition;
export {
  export_faGunSlash as faGunSlash
};
