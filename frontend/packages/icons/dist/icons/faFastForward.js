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

// src/node_modules/@fortawesome/free-solid-svg-icons/faForwardFast.js
var require_faForwardFast = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faForwardFast.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "forward-fast";
    var width = 512;
    var height = 512;
    var aliases = [9197, "fast-forward"];
    var unicode = "f050";
    var svgPathData = "M19.8 477.6c12 5 25.7 2.2 34.9-6.9L224 301.3 224 448c0 12.9 7.8 24.6 19.8 29.6s25.7 2.2 34.9-6.9L448 301.3 448 448c0 17.7 14.3 32 32 32s32-14.3 32-32l0-384c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 146.7-169.4-169.4c-9.2-9.2-22.9-11.9-34.9-6.9S224 51.1 224 64L224 210.7 54.6 41.4c-9.2-9.2-22.9-11.9-34.9-6.9S0 51.1 0 64L0 448c0 12.9 7.8 24.6 19.8 29.6z";
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
    exports.faForwardFast = exports.definition;
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

// src/_entries/faFastForward.js
var import_faForwardFast = __toESM(require_faForwardFast());
var export_faFastForward = import_faForwardFast.definition;
export {
  export_faFastForward as faFastForward
};
