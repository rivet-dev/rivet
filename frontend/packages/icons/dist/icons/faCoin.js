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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCoin.js
var require_faCoin = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCoin.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "coin";
    var width = 512;
    var height = 512;
    var aliases = [129689];
    var unicode = "f85c";
    var svgPathData = "M256 352C114.6 352 0 287.5 0 208S114.6 64 256 64s256 64.5 256 144s-114.6 144-256 144zM86.6 248.2c12 11.4 27.7 20.1 44.4 26.8c33.6 13.4 77.8 21 125 21s91.4-7.6 125-21c16.7-6.7 32.4-15.4 44.4-26.8C437.5 236.7 448 220.5 448 200s-10.5-36.7-22.6-48.2c-12-11.4-27.7-20.1-44.4-26.8c-33.6-13.4-77.8-21-125-21s-91.4 7.6-125 21c-16.7 6.7-32.4 15.4-44.4 26.8C74.5 163.3 64 179.5 64 200s10.5 36.7 22.6 48.2zM96 200c0-35.3 71.6-64 160-64s160 28.7 160 64s-71.6 64-160 64s-160-28.7-160-64zM0 290.1c13.2 15.6 29.7 29.3 48 40.9l0 63.7C17.8 373.6 0 347.9 0 320l0-29.9zM80 413l0-64.7c28.4 13.2 60.9 23 96 29l0 64.3c-36.2-6-68.9-15.9-96-28.7zm128 32.8l0-64.1c15.7 1.6 31.7 2.4 48 2.4s32.3-.8 48-2.4l0 64.1c-15.5 1.5-31.6 2.2-48 2.2s-32.5-.8-48-2.2zm128-4.1l0-64.3c35.1-6 67.6-15.9 96-29l0 64.7c-27.1 12.8-59.8 22.7-96 28.7zm128-47l0-63.7c18.3-11.5 34.8-25.2 48-40.9l0 29.9c0 27.9-17.8 53.6-48 74.6z";
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
    exports.faCoin = exports.definition;
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

// src/_entries/faCoin.js
var import_faCoin = __toESM(require_faCoin());
var export_faCoin = import_faCoin.definition;
export {
  export_faCoin as faCoin
};
