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

// src/node_modules/@fortawesome/free-brands-svg-icons/faForumbee.js
var require_faForumbee = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faForumbee.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "forumbee";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f211";
    var svgPathData = "M5.8 309.7C2 292.7 0 275.5 0 258.3 0 135 99.8 35 223.1 35 239.7 35 256.4 37 272.4 40.5 149 87.5 51.9 186 5.8 309.7zM398.7 120.5c-13.7-17.5-29.7-32.7-47.8-45.3-149.6 44.3-266.3 162.1-309.7 312 12.5 18.1 28 35.6 45.2 49 43.1-151.3 161.2-271.7 312.3-315.7zm15.8 252.7c15.2-25.1 25.4-53.7 29.5-82.8-79.4 42.9-145 110.6-187.6 190.3 30-4.4 58.9-15.3 84.6-31.3 35 13.1 70.9 24.3 107 33.6-9.3-36.5-20.4-74.5-33.5-109.8zm29.7-145.5c-2.6-19.5-7.9-38.7-15.8-56.8-137.9 45.8-246.4 156.6-291.3 295.1 18.1 7.6 37 12.5 56.6 15.2 46.3-114.1 136.8-206.8 250.5-253.5z";
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
    exports.faForumbee = exports.definition;
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

// src/_entries/faForumbee.js
var import_faForumbee = __toESM(require_faForumbee());
var export_faForumbee = import_faForumbee.definition;
export {
  export_faForumbee as faForumbee
};
