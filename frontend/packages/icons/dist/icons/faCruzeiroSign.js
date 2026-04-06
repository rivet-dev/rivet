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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCruzeiroSign.js
var require_faCruzeiroSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCruzeiroSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cruzeiro-sign";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e152";
    var svgPathData = "M240 96c-88.4 0-160 71.6-160 160 0 71.6 47.1 132.3 112 152.7L192 256c0-13.3 10.7-24 24-24 12.8 0 23.3 10.1 24 22.7 15.5-9.5 33.5-14.7 52.3-14.7l11.8 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-11.8 0c-18.8 0-36.1 10-45.4 26.3-4.5 7.9-6.9 16.8-6.9 25.9l0 75.8c42.5 0 81.1-16.6 109.8-43.6 12.9-12.1 33.1-11.5 45.2 1.3s11.5 33.1-1.3 45.2C353.7 456.8 299.5 480 240 480 116.3 480 16 379.7 16 256S116.3 32 240 32c59.5 0 113.7 23.2 153.7 61.1 12.9 12.1 13.4 32.4 1.3 45.2s-32.4 13.4-45.2 1.3C321.1 112.6 282.5 96 240 96z";
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
    exports.faCruzeiroSign = exports.definition;
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

// src/_entries/faCruzeiroSign.js
var import_faCruzeiroSign = __toESM(require_faCruzeiroSign());
var export_faCruzeiroSign = import_faCruzeiroSign.definition;
export {
  export_faCruzeiroSign as faCruzeiroSign
};
