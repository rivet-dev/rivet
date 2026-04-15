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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCameraViewfinder.js
var require_faCameraViewfinder = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCameraViewfinder.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "camera-viewfinder";
    var width = 512;
    var height = 512;
    var aliases = ["screenshot"];
    var unicode = "e0da";
    var svgPathData = "M56 0l80 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L56 48c-4.4 0-8 3.6-8 8l0 80c0 13.3-10.7 24-24 24s-24-10.7-24-24L0 56C0 25.1 25.1 0 56 0zM376 0l80 0c30.9 0 56 25.1 56 56l0 80c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-80c0-4.4-3.6-8-8-8l-80 0c-13.3 0-24-10.7-24-24s10.7-24 24-24zM48 376l0 80c0 4.4 3.6 8 8 8l80 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-80 0c-30.9 0-56-25.1-56-56l0-80c0-13.3 10.7-24 24-24s24 10.7 24 24zm464 0l0 80c0 30.9-25.1 56-56 56l-80 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l80 0c4.4 0 8-3.6 8-8l0-80c0-13.3 10.7-24 24-24s24 10.7 24 24zM180 128l6.2-16.4c3.5-9.4 12.5-15.6 22.5-15.6l94.7 0c10 0 19 6.2 22.5 15.6L332 128l36 0c26.5 0 48 21.5 48 48l0 160c0 26.5-21.5 48-48 48l-224 0c-26.5 0-48-21.5-48-48l0-160c0-26.5 21.5-48 48-48l36 0zM320 256a64 64 0 1 0 -128 0 64 64 0 1 0 128 0z";
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
    exports.faCameraViewfinder = exports.definition;
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

// src/_entries/faCameraViewfinder.js
var import_faCameraViewfinder = __toESM(require_faCameraViewfinder());
var export_faCameraViewfinder = import_faCameraViewfinder.definition;
export {
  export_faCameraViewfinder as faCameraViewfinder
};
