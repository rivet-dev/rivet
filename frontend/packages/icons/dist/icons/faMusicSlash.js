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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMusicSlash.js
var require_faMusicSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMusicSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "music-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f8d1";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7l-71.5-56C569.8 400.2 576 384.7 576 368l0-264 0-72c0-10.1-4.8-19.6-12.9-25.7s-18.6-7.9-28.3-5l-320 96c-12.7 3.8-21.6 14.9-22.7 27.9L38.8 5.1zM300.7 210.4L512 147l0 145.5c-10-2.9-20.8-4.6-32-4.6c-22.2 0-42.6 6.3-58.8 16.8L300.7 210.4zM192 247.3l0 109.3c-10-3-20.8-4.6-32-4.6c-53 0-96 35.8-96 80s43 80 96 80s96-35.8 96-80l0-134.3-64-50.4z";
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
    exports.faMusicSlash = exports.definition;
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

// src/_entries/faMusicSlash.js
var import_faMusicSlash = __toESM(require_faMusicSlash());
var export_faMusicSlash = import_faMusicSlash.definition;
export {
  export_faMusicSlash as faMusicSlash
};
