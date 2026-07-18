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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBowlSoftServe.js
var require_faBowlSoftServe = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBowlSoftServe.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bowl-soft-serve";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e46b";
    var svgPathData = "M257.2 9.9c2.5-6 8.3-9.9 14.8-9.9l36.6 0C367.9 0 416 48.1 416 107.4c0 19.9-6.9 38.1-18.3 52.6l18.3 0c35.3 0 64 28.7 64 64s-28.7 64-64 64L96 288c-35.3 0-64-28.7-64-64s28.7-64 64-64l8.6 0c-5.4-9.4-8.6-20.3-8.6-32c0-35.3 28.7-64 64-64l94.1 0C264 64 272 56 272 46.1c0-4.8-1.9-9.3-5.3-12.7l-6.1-6.1c-4.6-4.6-5.9-11.5-3.5-17.4zM64 320l384 0c8.8 0 16 7.2 16 16l0 8c0 57.4-46.6 104-104 104l-30.1 0 20.4 40.8c1.1 2.3 1.7 4.7 1.7 7.2c0 2.9-.8 5.8-2.4 8.4c-2.9 4.7-8.1 7.6-13.6 7.6l-160 0c-5.5 0-10.7-2.9-13.6-7.6c-1.6-2.6-2.4-5.5-2.4-8.4c0-2.4 .6-4.9 1.7-7.2L182.1 448 152 448C94.6 448 48 401.4 48 344l0-8c0-8.8 7.2-16 16-16z";
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
    exports.faBowlSoftServe = exports.definition;
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

// src/_entries/faBowlSoftServe.js
var import_faBowlSoftServe = __toESM(require_faBowlSoftServe());
var export_faBowlSoftServe = import_faBowlSoftServe.definition;
export {
  export_faBowlSoftServe as faBowlSoftServe
};
