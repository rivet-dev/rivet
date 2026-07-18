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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCandyBar.js
var require_faCandyBar = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCandyBar.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "candy-bar";
    var width = 640;
    var height = 512;
    var aliases = [127851, "chocolate-bar"];
    var unicode = "e3e8";
    var svgPathData = "M0 320l128 0 0-96L0 224l0 96zm0 64c0 35.3 28.7 64 64 64l64 0 0-96L0 352l0 32zm288 64l0-96-128 0 0 96 128 0zm0-224l-128 0 0 96 128 0 0-96zm0-160L160 64l0 128 128 0 0-128zm288 0L320 64l0 384 256 0c35.3 0 64-28.7 64-64l0-256c0-35.3-28.7-64-64-64zM44.2 192l75.8 0s0 0 0 0l8 0 0-32s0 0 0 0l0-51.8c0-7.1-9-11.1-15-7.4C101 107.9 87 112 72 112c-4 0-7.9-.3-11.7-.9c-7-1-14.2 6.2-13.1 13.1c.6 3.8 .9 7.7 .9 11.7c0 15-4.1 29-11.3 41c-3.6 6.1 .4 15 7.4 15z";
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
    exports.faCandyBar = exports.definition;
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

// src/_entries/faCandyBar.js
var import_faCandyBar = __toESM(require_faCandyBar());
var export_faCandyBar = import_faCandyBar.definition;
export {
  export_faCandyBar as faCandyBar
};
