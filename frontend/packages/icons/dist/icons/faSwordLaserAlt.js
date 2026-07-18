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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSwordLaserAlt.js
var require_faSwordLaserAlt = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSwordLaserAlt.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sword-laser-alt";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e03c";
    var svgPathData = "M505.7 40.2c8.7-9.5 8.3-24.1-.7-33.2s-23.7-9.4-33.2-.7L225.1 232l105.1 0L505.7 40.2zM189.3 256c-12.7 0-24.9 5.1-33.9 14.1l-49 49 27.3 27.3 32 32L193 405.7 315.3 283.3c4.6-4.6 5.9-11.5 3.5-17.4s-8.3-9.9-14.8-9.9l-114.7 0zm-7.6 161L160 395.3 54.3 501l3.5 3.5c4.8 4.8 11.3 7.5 18.1 7.5s13.3-2.7 18.1-7.5L181.7 417zM43 489.7L148.7 384 128 363.3 22.3 469 43 489.7zm-32-32L116.7 352 95 330.3 7.5 417.9C2.7 422.7 0 429.2 0 436s2.7 13.3 7.5 18.1l3.5 3.5z";
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
    exports.faSwordLaserAlt = exports.definition;
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

// src/_entries/faSwordLaserAlt.js
var import_faSwordLaserAlt = __toESM(require_faSwordLaserAlt());
var export_faSwordLaserAlt = import_faSwordLaserAlt.definition;
export {
  export_faSwordLaserAlt as faSwordLaserAlt
};
