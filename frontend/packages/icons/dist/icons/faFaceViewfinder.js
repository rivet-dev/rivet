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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceViewfinder.js
var require_faFaceViewfinder = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceViewfinder.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-viewfinder";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e2ff";
    var svgPathData = "M56 0l80 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L56 48c-4.4 0-8 3.6-8 8l0 80c0 13.3-10.7 24-24 24s-24-10.7-24-24L0 56C0 25.1 25.1 0 56 0zM376 0l80 0c30.9 0 56 25.1 56 56l0 80c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-80c0-4.4-3.6-8-8-8l-80 0c-13.3 0-24-10.7-24-24s10.7-24 24-24zM48 376l0 80c0 4.4 3.6 8 8 8l80 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-80 0c-30.9 0-56-25.1-56-56l0-80c0-13.3 10.7-24 24-24s24 10.7 24 24zm464 0l0 80c0 30.9-25.1 56-56 56l-80 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l80 0c4.4 0 8-3.6 8-8l0-80c0-13.3 10.7-24 24-24s24 10.7 24 24zM96 256a160 160 0 1 1 320 0A160 160 0 1 1 96 256zm128-40a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm88 24a24 24 0 1 0 0-48 24 24 0 1 0 0 48zM169.9 292.5c-6.4 6.1-6.6 16.2-.5 22.6C191.2 337.8 222 352 256 352s64.8-14.2 86.6-36.9c6.1-6.4 5.9-16.5-.5-22.6s-16.5-5.9-22.6 .5C303.5 309.6 281 320 256 320s-47.5-10.4-63.5-27.1c-6.1-6.4-16.2-6.6-22.6-.5z";
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
    exports.faFaceViewfinder = exports.definition;
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

// src/_entries/faFaceViewfinder.js
var import_faFaceViewfinder = __toESM(require_faFaceViewfinder());
var export_faFaceViewfinder = import_faFaceViewfinder.definition;
export {
  export_faFaceViewfinder as faFaceViewfinder
};
