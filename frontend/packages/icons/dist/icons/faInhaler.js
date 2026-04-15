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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faInhaler.js
var require_faInhaler = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faInhaler.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "inhaler";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f5f9";
    var svgPathData = "M429.8 32.7c-12.7-3.2-25.6 4.4-29 17l-14 51.8L533.2 248 575.2 94.3c1.7-6.2 .8-12.9-2.4-18.4s-8.6-9.6-14.9-11.2l-128-32zM534.1 305.5c1.5-5.5-.1-11.5-4.1-15.5L383.8 143.8c-8.7-8.7-23.6-4.7-26.8 7.2l-16.5 61.2c-1.9 7-8.2 11.8-15.4 11.8L232 224c-22.1 0-40 17.9-40 40l0 176c0 22.1 17.9 40 40 40l224 0c18 0 33.8-12.1 38.6-29.5l39.5-145zM64 256A32 32 0 1 0 0 256a32 32 0 1 0 64 0zm0 96A32 32 0 1 0 0 352a32 32 0 1 0 64 0zm64-16a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM64 448A32 32 0 1 0 0 448a32 32 0 1 0 64 0zm64-16a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
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
    exports.faInhaler = exports.definition;
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

// src/_entries/faInhaler.js
var import_faInhaler = __toESM(require_faInhaler());
var export_faInhaler = import_faInhaler.definition;
export {
  export_faInhaler as faInhaler
};
