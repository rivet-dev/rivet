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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTricycleAdult.js
var require_faTricycleAdult = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTricycleAdult.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tricycle-adult";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e5c4";
    var svgPathData = "M312 64c-13.3 0-24 10.7-24 24s10.7 24 24 24l31.4 0c2.6 0 5 1.3 6.5 3.4l79.2 112.5-48.2 64.3c-5.6 7.5-14.4 11.9-23.7 11.9c-1.9 0-3.8-.2-5.7-.5l-322.9-63c-13-2.5-25.6 6-28.2 19s6 25.6 19 28.2l4.6 .9C9.1 305.4 0 327.6 0 352c0 53 43 96 96 96c18.2 0 35.3-5.1 49.8-13.9c-.6 .4-1.2 .7-1.8 1.1c14.1 8.2 30.5 12.8 48 12.8c53 0 96-43 96-96c0-4.1-.3-8.2-.8-12.2l55.1 10.7c4.9 1 9.9 1.4 14.9 1.4c24.4 0 47.4-11.5 62.1-31.1l38.8-51.8 13.7 19.5C457 305.6 448 327.7 448 352c0 53 43 96 96 96s96-43 96-96s-43-96-96-96c-11.4 0-22.3 2-32.5 5.6L389.2 87.8C378.7 72.9 361.6 64 343.4 64L312 64zM223.5 327.4c5.3 6.8 8.5 15.3 8.5 24.6c0 22.1-17.9 40-40 40c-2.9 0-5.6-.3-8.3-.9c5.3-12 8.3-25.2 8.3-39.1c0-11.2-1.9-21.9-5.4-31.8l36.9 7.2zM96 312a40 40 0 1 1 0 80 40 40 0 1 1 0-80zm448 0a40 40 0 1 1 0 80 40 40 0 1 1 0-80zM94.1 117.2c-6-16.6-24.3-25.3-40.9-19.3s-25.3 24.3-19.3 40.9l14.9 41.7c9.4 26.2 31.7 45.6 58.9 51.3l77.7 16.3c17.3 3.6 34.3-7.4 37.9-24.7s-7.4-34.3-24.7-37.9l-77.7-16.3c-5.4-1.1-9.9-5-11.8-10.3L94.1 117.2z";
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
    exports.faTricycleAdult = exports.definition;
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

// src/_entries/faTricycleAdult.js
var import_faTricycleAdult = __toESM(require_faTricycleAdult());
var export_faTricycleAdult = import_faTricycleAdult.definition;
export {
  export_faTricycleAdult as faTricycleAdult
};
