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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBalloons.js
var require_faBalloons = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBalloons.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "balloons";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e2e4";
    var svgPathData = "M320 160C320 71.6 248.4 0 160 0S0 71.6 0 160c0 92.5 87.2 174 117.7 199.7c5.9 5 7.8 13.4 4.4 20.3L97.4 429.3c-.9 1.8-1.4 3.8-1.4 5.8c0 7.1 5.8 12.9 12.9 12.9l102.1 0c7.1 0 12.9-5.8 12.9-12.9c0-2-.5-4-1.4-5.8L198 379.9c-3.5-6.9-1.6-15.3 4.4-20.3C232.8 334 320 252.5 320 160zM144 96c-26.5 0-48 21.5-48 48c0 8.8-7.2 16-16 16s-16-7.2-16-16c0-44.2 35.8-80 80-80c8.8 0 16 7.2 16 16s-7.2 16-16 16zM437.7 423.7c5.9 5 7.8 13.4 4.4 20.3l-24.7 49.3c-.9 1.8-1.4 3.8-1.4 5.8c0 7.1 5.8 12.9 12.9 12.9l102.1 0c7.1 0 12.9-5.8 12.9-12.9c0-2-.5-4-1.4-5.8L518 443.9c-3.5-6.9-1.6-15.3 4.4-20.3C552.8 398 640 316.5 640 224c0-88.4-71.6-160-160-160c-53.7 0-101.2 26.4-130.2 67c1.4 9.5 2.2 19.2 2.2 29c0 37.2-11.5 71.8-27.3 101.9c18.5 76.2 86.8 139.7 113 161.8zM416 208c0 8.8-7.2 16-16 16s-16-7.2-16-16c0-44.2 35.8-80 80-80c8.8 0 16 7.2 16 16s-7.2 16-16 16c-26.5 0-48 21.5-48 48z";
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
    exports.faBalloons = exports.definition;
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

// src/_entries/faBalloons.js
var import_faBalloons = __toESM(require_faBalloons());
var export_faBalloons = import_faBalloons.definition;
export {
  export_faBalloons as faBalloons
};
