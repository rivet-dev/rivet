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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMobileNotch.js
var require_faMobileNotch = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMobileNotch.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mobile-notch";
    var width = 384;
    var height = 512;
    var aliases = ["mobile-iphone"];
    var unicode = "e1ee";
    var svgPathData = "M128 64l0 16c0 8.8 7.2 16 16 16l96 0c8.8 0 16-7.2 16-16l0-16 32 0c8.8 0 16 7.2 16 16l0 352c0 8.8-7.2 16-16 16L96 448c-8.8 0-16-7.2-16-16L80 80c0-8.8 7.2-16 16-16l32 0zM96 0C51.8 0 16 35.8 16 80l0 352c0 44.2 35.8 80 80 80l192 0c44.2 0 80-35.8 80-80l0-352c0-44.2-35.8-80-80-80L96 0zm32 400c0 8.8 7.2 16 16 16l96 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-96 0c-8.8 0-16 7.2-16 16z";
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
    exports.faMobileNotch = exports.definition;
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

// src/_entries/faMobileIphone.js
var import_faMobileNotch = __toESM(require_faMobileNotch());
var export_faMobileIphone = import_faMobileNotch.definition;
export {
  export_faMobileIphone as faMobileIphone
};
