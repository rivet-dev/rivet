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

// src/node_modules/@fortawesome/free-solid-svg-icons/faInfinity.js
var require_faInfinity = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faInfinity.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "infinity";
    var width = 640;
    var height = 512;
    var aliases = [8734, 9854];
    var unicode = "f534";
    var svgPathData = "M0 256c0-88.4 71.6-160 160-160 50.4 0 97.8 23.7 128 64l32 42.7 32-42.7c30.2-40.3 77.6-64 128-64 88.4 0 160 71.6 160 160S568.4 416 480 416c-50.4 0-97.8-23.7-128-64l-32-42.7-32 42.7c-30.2 40.3-77.6 64-128 64-88.4 0-160-71.6-160-160zm280 0l-43.2-57.6c-18.1-24.2-46.6-38.4-76.8-38.4-53 0-96 43-96 96s43 96 96 96c30.2 0 58.7-14.2 76.8-38.4L280 256zm80 0l43.2 57.6c18.1 24.2 46.6 38.4 76.8 38.4 53 0 96-43 96-96s-43-96-96-96c-30.2 0-58.7 14.2-76.8 38.4L360 256z";
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
    exports.faInfinity = exports.definition;
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

// src/_entries/faInfinity.js
var import_faInfinity = __toESM(require_faInfinity());
var export_faInfinity = import_faInfinity.definition;
export {
  export_faInfinity as faInfinity
};
