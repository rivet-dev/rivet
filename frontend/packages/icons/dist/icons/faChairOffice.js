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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faChairOffice.js
var require_faChairOffice = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faChairOffice.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chair-office";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f6c1";
    var svgPathData = "M160 0L288 0c35.3 0 64 28.7 64 64l0 192L96 256 96 64c0-35.3 28.7-64 64-64zM64 160l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64c0-17.7 14.3-32 32-32s32 14.3 32 32zm384 0l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64c0-17.7 14.3-32 32-32s32 14.3 32 32zM51.4 305.7C56.8 294.8 67.9 288 80 288l288 0c12.1 0 23.2 6.8 28.6 17.7l16 32c5 9.9 4.4 21.7-1.4 31.1s-16.1 15.2-27.2 15.2l-128 0 0 64 32 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-64 0-64 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l32 0 0-64L64 384c-11.1 0-21.4-5.7-27.2-15.2s-6.4-21.2-1.4-31.1l16-32z";
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
    exports.faChairOffice = exports.definition;
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

// src/_entries/faChairOffice.js
var import_faChairOffice = __toESM(require_faChairOffice());
var export_faChairOffice = import_faChairOffice.definition;
export {
  export_faChairOffice as faChairOffice
};
