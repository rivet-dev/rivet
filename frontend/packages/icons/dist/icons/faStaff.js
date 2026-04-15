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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faStaff.js
var require_faStaff = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faStaff.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "staff";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f71b";
    var svgPathData = "M284.2 44.2C297.8 17.1 325.5 0 355.8 0L432 0c44.2 0 80 35.8 80 80l0 103.9c0 35.7-23.7 67.1-58 76.9l-3.9 1.1c-4 1.2-7.5 3.9-9.5 7.5l-19 33.6c-4.3 7.7-14.1 10.4-21.8 6.1l-32.8-18.5c-3.7-2.1-8.1-2.6-12.3-1.5l-85.5 24.4c-2.6 .7-5 2.1-6.9 4.1L81.9 497.9c-18.7 18.7-49.1 18.7-67.9 0s-18.7-49.1 0-67.9l77.3-77.3c3-3 4.7-7.1 4.7-11.3L96 304c0-8.8 7.2-16 16-16l37.5 0c4.2 0 8.3-1.7 11.3-4.7l39.4-39.4c9.6-9.6 21.5-16.6 34.6-20.4l169.6-48.5c6.9-2 11.6-8.2 11.6-15.4l0-47.7c0-8.8-7.2-16-16-16l-28.2 0c-3.7 0-7.2 2.1-8.8 5.5c-11.9 23.7-40.7 33.3-64.4 21.5s-33.3-40.7-21.5-64.4l7.2-14.3z";
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
    exports.faStaff = exports.definition;
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

// src/_entries/faStaff.js
var import_faStaff = __toESM(require_faStaff());
var export_faStaff = import_faStaff.definition;
export {
  export_faStaff as faStaff
};
