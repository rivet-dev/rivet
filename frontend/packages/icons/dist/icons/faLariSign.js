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

// src/node_modules/@fortawesome/free-solid-svg-icons/faLariSign.js
var require_faLariSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faLariSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lari-sign";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e1c8";
    var svgPathData = "M144 0c13.3 0 24 10.7 24 24l0 41.5c7.9-1 15.9-1.5 24-1.5s16.1 .5 24 1.5L216 24c0-13.3 10.7-24 24-24s24 10.7 24 24l0 54c58.9 23.8 103.2 76 116.2 139.7 3.5 17.3-7.7 34.2-25 37.7s-34.2-7.7-37.7-25c-6.7-33.2-26.4-61.8-53.4-80.2l0 81.8c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-101.8c-7.8-1.5-15.8-2.2-24-2.2s-16.2 .8-24 2.2L168 232c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-81.8c-33.8 23-56 61.9-56 105.8 0 70.7 57.3 128 128 128l160 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L32 448c-17.7 0-32-14.3-32-32s14.3-32 32-32l16.9 0C18.5 350 0 305.2 0 256 0 175.4 49.6 106.4 120 78l0-54c0-13.3 10.7-24 24-24z";
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
    exports.faLariSign = exports.definition;
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

// src/_entries/faLariSign.js
var import_faLariSign = __toESM(require_faLariSign());
var export_faLariSign = import_faLariSign.definition;
export {
  export_faLariSign as faLariSign
};
