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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSymbols.js
var require_faSymbols = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSymbols.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "symbols";
    var width = 512;
    var height = 512;
    var aliases = [128291, "icons-alt"];
    var unicode = "f86e";
    var svgPathData = "M500.3 7.3C507.7 13.3 512 22.4 512 32l0 144c0 26.5-28.7 48-64 48s-64-21.5-64-48s28.7-48 64-48l0-57L352 90.2 352 208c0 26.5-28.7 48-64 48s-64-21.5-64-48s28.7-48 64-48l0-96c0-15.3 10.8-28.4 25.7-31.4l160-32c9.4-1.9 19.1 .6 26.6 6.6zM32 0L192 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L32 64C14.3 64 0 49.7 0 32S14.3 0 32 0zM502.6 342.6l-160 160c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3l160-160c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3zM0 128c0-17.7 14.3-32 32-32l80 0 80 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-48 0 0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32-48 0c-17.7 0-32-14.3-32-32zM288 328a40 40 0 1 1 80 0 40 40 0 1 1 -80 0zM472 432a40 40 0 1 1 0 80 40 40 0 1 1 0-80zM135.6 374l10.3 10.8 23.5-23.5c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3l-24.7 24.7L215.2 458c12.2 12.8 11.6 33.1-1.2 45.2s-33.1 11.6-45.2-1.2l-24.1-25.4-12.8 12.8C117.4 503.9 97.7 512 77.3 512C34.6 512 0 477.4 0 434.7l0-.9c0-19.4 7.2-38.2 20.3-52.6l13.1-14.4C22.5 354.3 16 337.9 16 320c0-39.8 32.2-72 72-72s72 32.2 72 72c0 21.5-9.4 40.8-24.4 54zm-68 50.3c-2.4 2.6-3.7 6-3.7 9.5l0 .9c0 7.3 5.9 13.3 13.3 13.3c3.5 0 6.9-1.4 9.4-3.9l14-14L81.1 409.5 67.7 424.3zM104 320a16 16 0 1 0 -32 0 16 16 0 1 0 32 0z";
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
    exports.faSymbols = exports.definition;
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

// src/_entries/faSymbols.js
var import_faSymbols = __toESM(require_faSymbols());
var export_faSymbols = import_faSymbols.definition;
export {
  export_faSymbols as faSymbols
};
