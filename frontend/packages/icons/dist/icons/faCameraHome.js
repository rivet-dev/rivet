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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCameraSecurity.js
var require_faCameraSecurity = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCameraSecurity.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "camera-security";
    var width = 448;
    var height = 512;
    var aliases = ["camera-home"];
    var unicode = "f8fe";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L384 0c35.3 0 64 28.7 64 64l0 288c0 35.3-28.7 64-64 64l-74 0 59 36.9c12.1 7.6 17.7 22.2 13.8 36S366.3 512 352 512L96 512c-14.3 0-26.8-9.5-30.8-23.2s1.7-28.4 13.8-36L138 416l-74 0c-35.3 0-64-28.7-64-64L0 64zM128 208a96 96 0 1 1 192 0 96 96 0 1 1 -192 0zm224 0A128 128 0 1 0 96 208a128 128 0 1 0 256 0z";
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
    exports.faCameraSecurity = exports.definition;
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

// src/_entries/faCameraHome.js
var import_faCameraSecurity = __toESM(require_faCameraSecurity());
var export_faCameraHome = import_faCameraSecurity.definition;
export {
  export_faCameraHome as faCameraHome
};
