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

// src/node_modules/@fortawesome/free-brands-svg-icons/faFoursquare.js
var require_faFoursquare = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faFoursquare.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "foursquare";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f180";
    var svgPathData = "M331.1 3L57.9 3C20.4 3 8 31.3 8 49.1L8 482.9c0 20.3 12.1 27.7 18.2 30.1 6.2 2.5 22.8 4.6 32.9-7.1 128.9-149.4 131.1-151.9 131.1-151.9 3.1-3.4 3.4-3.1 6.8-3.1l83.4 0c35.1 0 40.6-25.2 44.3-39.7l48.6-243C381.8 25.8 371.1 3 331.1 3zM314.8 76.8l-11.4 59.7c-1.2 6.5-9.5 13.2-16.9 13.2l-106.4 0c-12 0-20.6 8.3-20.6 20.3l0 13c0 12 8.6 20.6 20.6 20.6l90.4 0c8.3 0 16.6 9.2 14.8 18.2-1.8 8.9-10.5 53.8-11.4 58.8-.9 4.9-6.8 13.5-16.9 13.5l-73.5 0c-13.5 0-17.2 1.8-26.5 12.6 0 0-8.9 11.4-89.5 108.3-.9 .9-1.8 .6-1.8-.3l0-338.8c0-7.7 6.8-16.6 16.6-16.6l219 0c8.2 0 15.6 7.7 13.5 17.5z";
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
    exports.faFoursquare = exports.definition;
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

// src/_entries/faFoursquare.js
var import_faFoursquare = __toESM(require_faFoursquare());
var export_faFoursquare = import_faFoursquare.definition;
export {
  export_faFoursquare as faFoursquare
};
