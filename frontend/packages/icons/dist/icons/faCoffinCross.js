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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCoffinCross.js
var require_faCoffinCross = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCoffinCross.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "coffin-cross";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e051";
    var svgPathData = "M102.4 10.9l-91 104C4.1 123.3 0 134.2 0 145.4c0 4.4 .6 8.7 1.8 12.9L97.3 488.9c4 13.7 16.5 23.1 30.7 23.1l127.9 0c14.3 0 26.8-9.4 30.7-23.1l95.5-330.6c1.2-4.2 1.8-8.5 1.8-12.9c0-11.2-4.1-22.1-11.5-30.6l-91-104C275.5 4 266.7 0 257.5 0l-131 0c-9.2 0-18 4-24.1 10.9zM216 120l0 40 48 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-48 0 0 120c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-120-48 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l48 0 0-40c0-13.3 10.7-24 24-24s24 10.7 24 24z";
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
    exports.faCoffinCross = exports.definition;
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

// src/_entries/faCoffinCross.js
var import_faCoffinCross = __toESM(require_faCoffinCross());
var export_faCoffinCross = import_faCoffinCross.definition;
export {
  export_faCoffinCross as faCoffinCross
};
