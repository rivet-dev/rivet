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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBadgeDollar.js
var require_faBadgeDollar = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBadgeDollar.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "badge-dollar";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f645";
    var svgPathData = "M256 0c36.8 0 68.8 20.7 84.9 51.1C373.8 41 411 49 437 75s34 63.3 23.9 96.1C491.3 187.2 512 219.2 512 256s-20.7 68.8-51.1 84.9C471 373.8 463 411 437 437s-63.3 34-96.1 23.9C324.8 491.3 292.8 512 256 512s-68.8-20.7-84.9-51.1C138.2 471 101 463 75 437s-34-63.3-23.9-96.1C20.7 324.8 0 292.8 0 256s20.7-68.8 51.1-84.9C41 138.2 49 101 75 75s63.3-34 96.1-23.9C187.2 20.7 219.2 0 256 0zm20.9 140.4c0-11-9-20-20-20s-20 9-20 20l0 17.4c-8.8 1.8-17.7 4.8-25.7 9.6c-15 8.9-27.7 24.4-27.6 46.9c.1 21.5 12.6 35.1 26.3 43.4c11.9 7.2 27 11.7 39.1 15.4l1.9 .6c13.9 4.2 24.4 7.6 31.7 12.2c6.1 3.8 7.5 6.8 7.6 11c.1 6.5-2.5 10.4-7.6 13.6c-6 3.7-15.2 5.9-25.1 5.6c-12.7-.4-24.6-4.4-39.7-9.6c0 0 0 0 0 0s0 0 0 0s0 0 0 0s0 0 0 0c-2.6-.9-5.2-1.8-8-2.7c-10.5-3.5-21.8 2.2-25.3 12.6s2.2 21.8 12.6 25.3c2.2 .7 4.5 1.5 6.8 2.3c9.7 3.3 20.9 7.2 32.8 9.6l0 18c0 11 9 20 20 20s20-9 20-20l0-17.1c9.3-1.7 18.6-4.9 26.8-10.1c15.3-9.5 26.8-25.7 26.5-48.1c-.3-21.6-12.4-35.6-26.3-44.3c-12.5-7.9-28.3-12.6-40.8-16.4l-.6-.2c-14.1-4.3-24.6-7.5-31.9-11.9c-6.2-3.7-7-6.3-7-9.3c0-5.1 2.1-8.8 7.9-12.3c6.5-3.8 16-6 25-5.8c10.9 .2 22.9 2.6 35.2 5.8c10.7 2.8 21.6-3.5 24.5-14.2s-3.5-21.6-14.2-24.5c-7.8-2.1-16.3-4.1-25.1-5.4l0-17.3z";
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
    exports.faBadgeDollar = exports.definition;
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

// src/_entries/faBadgeDollar.js
var import_faBadgeDollar = __toESM(require_faBadgeDollar());
var export_faBadgeDollar = import_faBadgeDollar.definition;
export {
  export_faBadgeDollar as faBadgeDollar
};
