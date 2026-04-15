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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCoinBlank.js
var require_faCoinBlank = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCoinBlank.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "coin-blank";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3fb";
    var svgPathData = "M0 208c0 79.5 114.6 144 256 144s256-64.5 256-144s-114.6-144-256-144S0 128.5 0 208zm0 82.1L0 320c0 27.9 17.8 53.6 48 74.6L48 331c-18.3-11.5-34.8-25.2-48-40.9zM80 413c27.1 12.8 59.8 22.7 96 28.7l0-64.3c-35.1-6-67.6-15.9-96-29L80 413zm128 32.8c15.5 1.5 31.6 2.2 48 2.2s32.5-.8 48-2.2l0-64.1c-15.7 1.6-31.7 2.4-48 2.4s-32.3-.8-48-2.4l0 64.1zm128-4.1c36.2-6 68.9-15.9 96-28.7l0-64.7c-28.4 13.2-60.9 23-96 29l0 64.3zm128-47c30.2-21 48-46.8 48-74.6l0-29.9c-13.2 15.6-29.7 29.3-48 40.9l0 63.7z";
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
    exports.faCoinBlank = exports.definition;
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

// src/_entries/faCoinBlank.js
var import_faCoinBlank = __toESM(require_faCoinBlank());
var export_faCoinBlank = import_faCoinBlank.definition;
export {
  export_faCoinBlank as faCoinBlank
};
