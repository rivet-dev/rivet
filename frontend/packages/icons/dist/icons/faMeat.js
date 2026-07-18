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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMeat.js
var require_faMeat = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMeat.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "meat";
    var width = 512;
    var height = 512;
    var aliases = [127830];
    var unicode = "f814";
    var svgPathData = "M243.6 19.6C283.7-20.4 371.9 2.8 440.5 71.5s91.9 156.8 51.8 196.9l-40.5 40.5C403.9 357 338.7 384 270.8 384l-58.2 0c-7.1 0-13.9 2.8-18.9 7.8c-10.3 10.3-9.7 26.9-5 40.7c2.1 6.1 3.2 12.7 3.2 19.5c0 33.1-26.9 60-60 60s-60-26.9-60-60c0-6.3-5.7-12-12-12c-33.1 0-60-26.9-60-60s26.9-60 60-60c6.8 0 13.4 1.1 19.5 3.2c13.8 4.7 30.4 5.3 40.7-5c5-5 7.8-11.8 7.8-18.9l0-58.1c0-67.9 27-133 75-181l40.7-40.7zm64 184.8c49.6 49.6 113.3 66.4 142.2 37.4s12.2-92.6-37.4-142.2S299.1 33.2 270.2 62.2s-12.2 92.6 37.4 142.2zm36.3-36.3c-15.3-15.3-20.4-34.9-11.5-43.8s28.5-3.7 43.8 11.5s20.4 34.9 11.5 43.8s-28.5 3.7-43.8-11.5z";
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
    exports.faMeat = exports.definition;
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

// src/_entries/faMeat.js
var import_faMeat = __toESM(require_faMeat());
var export_faMeat = import_faMeat.definition;
export {
  export_faMeat as faMeat
};
