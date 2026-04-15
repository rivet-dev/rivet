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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBone.js
var require_faBone = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBone.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bone";
    var width = 640;
    var height = 512;
    var aliases = [129460];
    var unicode = "f5d7";
    var svgPathData = "M197.4 160c-3.9 0-7.2-2.8-8.1-6.6-10.2-42.1-48.1-73.4-93.3-73.4-53 0-96 43-96 96 0 29.1 12.9 55.1 33.3 72.7 4.3 3.7 4.3 10.8 0 14.5-20.4 17.6-33.3 43.7-33.3 72.7 0 53 43 96 96 96 45.2 0 83.1-31.3 93.3-73.4 .9-3.8 4.2-6.6 8.1-6.6l245.1 0c3.9 0 7.2 2.8 8.1 6.6 10.2 42.1 48.1 73.4 93.3 73.4 53 0 96-43 96-96 0-29.1-12.9-55.1-33.3-72.7-4.3-3.7-4.3-10.8 0-14.5 20.4-17.6 33.3-43.7 33.3-72.7 0-53-43-96-96-96-45.2 0-83.1 31.3-93.3 73.4-.9 3.8-4.2 6.6-8.1 6.6l-245.1 0z";
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
    exports.faBone = exports.definition;
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

// src/_entries/faBone.js
var import_faBone = __toESM(require_faBone());
var export_faBone = import_faBone.definition;
export {
  export_faBone as faBone
};
