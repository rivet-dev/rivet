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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTents.js
var require_faTents = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTents.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tents";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e582";
    var svgPathData = "M539.9 352l-95.6 0-8.4-66.1c-2.9-23.2-15.9-43.9-35.4-56.8L252.2 131.5c-18-11.9-39.7-15.7-60.1-11.5 2.5-4.3 5.9-7.9 10.1-10.7L350.6 11.6c10.7-7 24.5-7 35.2 0l148.4 97.7c7.8 5.1 13 13.4 14.2 22.7l23.3 184c2.4 19.1-12.5 36-31.7 36zM4.8 476l23.3-184c1.2-9.3 6.3-17.6 14.2-22.7l148.4-97.7c10.7-7 24.5-7 35.2 0l148.4 97.7c7.8 5.1 13 13.4 14.2 22.7l23.3 184c2.4 19.1-12.5 36-31.7 36l-49.7 0c-11.2 0-21.5-5.8-27.3-15.4l-77-126.7c-1.7-2.8-4.8-4.6-8.1-4.6-5.3 0-9.5 4.3-9.5 9.5l0 105.2c0 17.7-14.3 32-32 32L36.5 512c-19.3 0-34.2-16.9-31.7-36z";
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
    exports.faTents = exports.definition;
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

// src/_entries/faTents.js
var import_faTents = __toESM(require_faTents());
var export_faTents = import_faTents.definition;
export {
  export_faTents as faTents
};
